use alloc::string::String;
use alloc::vec::Vec;
use crate::net::tls::record::{TlsRecordCipher, CONTENT_TYPE_HANDSHAKE};
use crate::net::tls::sha256::{derive_secret, hkdf_expand_label, hkdf_extract, hmac_sha256, Sha256};
use crate::net::tls::x25519::{generate_keypair, x25519};

pub struct HandshakeBuilder {
    pub hostname: String,
    pub client_priv: [u8; 32],
    pub client_pub: [u8; 32],
    pub client_random: [u8; 32],
    pub transcript: Sha256,
}

pub struct ServerHelloInfo {
    pub server_pub: [u8; 32],
    pub cipher_suite: u16,
}

impl HandshakeBuilder {
    pub fn new(hostname: &str) -> Self {
        let (client_priv, client_pub) = generate_keypair();

        let ticks = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
        let mut client_random = [0u8; 32];
        let mut seed = ticks.wrapping_mul(0x5851f42d4c957f2d).wrapping_add(0x14057b7ef767814f);
        for i in 0..4 {
            seed = seed.wrapping_mul(0x5851f42d4c957f2d).wrapping_add(1);
            client_random[i * 8..(i + 1) * 8].copy_from_slice(&seed.to_le_bytes());
        }

        HandshakeBuilder {
            hostname: String::from(hostname),
            client_priv,
            client_pub,
            client_random,
            transcript: Sha256::new(),
        }
    }

    pub fn build_client_hello(&mut self) -> Vec<u8> {
        let mut ch_body = Vec::new();

        // 1. legacy_version = 0x0303
        ch_body.extend_from_slice(&[0x03, 0x03]);

        // 2. random = 32 bytes
        ch_body.extend_from_slice(&self.client_random);

        // 3. legacy_session_id = 32 bytes
        ch_body.push(32);
        ch_body.extend_from_slice(&self.client_random);

        // 4. cipher_suites = [TLS_CHACHA20_POLY1305_SHA256 (0x1303)]
        ch_body.extend_from_slice(&[0x00, 0x02, 0x13, 0x03]);

        // 5. legacy_compression_methods = [0x00]
        ch_body.extend_from_slice(&[0x01, 0x00]);

        // 6. extensions
        let mut extensions = Vec::new();

        // Extension: server_name (SNI) = 0x0000
        let host_bytes = self.hostname.as_bytes();
        let mut sni_ext = Vec::new();
        let sni_list_len = (host_bytes.len() + 3) as u16;
        sni_ext.extend_from_slice(&sni_list_len.to_be_bytes());
        sni_ext.push(0x00); // host_name type
        sni_ext.extend_from_slice(&(host_bytes.len() as u16).to_be_bytes());
        sni_ext.extend_from_slice(host_bytes);

        extensions.extend_from_slice(&[0x00, 0x00]);
        extensions.extend_from_slice(&(sni_ext.len() as u16).to_be_bytes());
        extensions.extend_from_slice(&sni_ext);

        // Extension: supported_versions = 0x002b
        extensions.extend_from_slice(&[0x00, 0x2b, 0x00, 0x03, 0x02, 0x03, 0x04]);

        // Extension: supported_groups = 0x000a (x25519 0x001d)
        extensions.extend_from_slice(&[0x00, 0x0a, 0x00, 0x04, 0x00, 0x02, 0x00, 0x1d]);

        // Extension: key_share = 0x0033
        let mut key_share = Vec::new();
        let client_shares_len = 36u16;
        key_share.extend_from_slice(&client_shares_len.to_be_bytes());
        key_share.extend_from_slice(&[0x00, 0x1d]);
        key_share.extend_from_slice(&32u16.to_be_bytes());
        key_share.extend_from_slice(&self.client_pub);

        extensions.extend_from_slice(&[0x00, 0x33]);
        extensions.extend_from_slice(&(key_share.len() as u16).to_be_bytes());
        extensions.extend_from_slice(&key_share);

        // Extension: signature_algorithms = 0x000d
        extensions.extend_from_slice(&[0x00, 0x0d, 0x00, 0x08, 0x00, 0x06, 0x08, 0x04, 0x04, 0x03, 0x04, 0x01]);

        // Extension: alpn = 0x0010 (http/1.1)
        extensions.extend_from_slice(&[0x00, 0x10, 0x00, 0x0b, 0x00, 0x09, 0x08, b'h', b't', b't', b'p', b'/', b'1', b'.', b'1']);

        ch_body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        ch_body.extend_from_slice(&extensions);

        // Wrap as Handshake message
        let mut handshake_msg = Vec::new();
        handshake_msg.push(0x01); // ClientHello
        let body_len = ch_body.len();
        handshake_msg.push((body_len >> 16) as u8);
        handshake_msg.push((body_len >> 8) as u8);
        handshake_msg.push((body_len & 0xff) as u8);
        handshake_msg.extend_from_slice(&ch_body);

        self.transcript.update(&handshake_msg);

        // Wrap as TLS record
        let mut record = Vec::new();
        record.push(CONTENT_TYPE_HANDSHAKE);
        record.extend_from_slice(&[0x03, 0x01]);
        record.extend_from_slice(&(handshake_msg.len() as u16).to_be_bytes());
        record.extend_from_slice(&handshake_msg);

        record
    }

    pub fn parse_server_hello(&mut self, sh_bytes: &[u8]) -> Result<ServerHelloInfo, &'static str> {
        if sh_bytes.len() < 38 {
            return Err("ServerHello too short");
        }

        if sh_bytes[0] != 0x02 {
            return Err("Expected ServerHello handshake type (0x02)");
        }

        let sh_len = ((sh_bytes[1] as usize) << 16) | ((sh_bytes[2] as usize) << 8) | (sh_bytes[3] as usize);
        if sh_bytes.len() < 4 + sh_len {
            return Err("ServerHello message truncated");
        }

        // Feed full ServerHello handshake message into transcript hash
        self.transcript.update(&sh_bytes[..4 + sh_len]);

        let body = &sh_bytes[4..4 + sh_len];
        let mut offset = 34; // skip legacy_version (2) and random (32)

        if offset >= body.len() {
            return Err("Truncated ServerHello body");
        }
        let session_id_len = body[offset] as usize;
        offset += 1 + session_id_len;

        if offset + 2 > body.len() {
            return Err("Truncated ServerHello cipher suite");
        }
        let cipher_suite = u16::from_be_bytes([body[offset], body[offset + 1]]);
        offset += 2;
        if cipher_suite != 0x1303 {
            return Err("Server did not select TLS_CHACHA20_POLY1305_SHA256");
        }

        if offset >= body.len() {
            return Err("Truncated ServerHello compression");
        }
        offset += 1; // skip compression method

        if offset + 2 > body.len() {
            return Err("Missing ServerHello extensions");
        }
        let ext_total_len = u16::from_be_bytes([body[offset], body[offset + 1]]) as usize;
        offset += 2;

        let ext_end = offset + ext_total_len;
        if ext_end > body.len() {
            return Err("ServerHello extensions truncated");
        }

        let mut server_pub: Option<[u8; 32]> = None;

        while offset + 4 <= ext_end {
            let ext_type = u16::from_be_bytes([body[offset], body[offset + 1]]);
            let ext_len = u16::from_be_bytes([body[offset + 2], body[offset + 3]]) as usize;
            offset += 4;

            if offset + ext_len > ext_end {
                return Err("Extension payload truncated");
            }

            let ext_data = &body[offset..offset + ext_len];
            offset += ext_len;

            if ext_type == 0x0033 {
                // key_share
                if ext_data.len() >= 36 {
                    let group = u16::from_be_bytes([ext_data[0], ext_data[1]]);
                    let key_len = u16::from_be_bytes([ext_data[2], ext_data[3]]) as usize;
                    if group == 0x001d && key_len == 32 && ext_data.len() >= 4 + 32 {
                        let mut sp = [0u8; 32];
                        sp.copy_from_slice(&ext_data[4..36]);
                        server_pub = Some(sp);
                    }
                }
            }
        }

        let sp = server_pub.ok_or("Server did not provide x25519 key_share")?;
        Ok(ServerHelloInfo {
            server_pub: sp,
            cipher_suite,
        })
    }

    pub fn compute_handshake_secrets(&self, server_pub: &[u8; 32]) -> (TlsRecordCipher, TlsRecordCipher, [u8; 32], [u8; 32]) {
        let shared_secret = x25519(&self.client_priv, server_pub);

        let empty_hash = Sha256::digest(b"");
        let early_secret = hkdf_extract(&[0u8; 32], &[0u8; 32]);
        let derived_1 = derive_secret(&early_secret, "derived", &empty_hash);
        let handshake_secret = hkdf_extract(&derived_1, &shared_secret);

        let digest_sh = self.transcript.clone().finalize();

        let c_hs = derive_secret(&handshake_secret, "c hs traffic", &digest_sh);
        let s_hs = derive_secret(&handshake_secret, "s hs traffic", &digest_sh);

        let c_hs_key_vec = hkdf_expand_label(&c_hs, "key", b"", 32);
        let c_hs_iv_vec = hkdf_expand_label(&c_hs, "iv", b"", 12);
        let s_hs_key_vec = hkdf_expand_label(&s_hs, "key", b"", 32);
        let s_hs_iv_vec = hkdf_expand_label(&s_hs, "iv", b"", 12);

        let mut c_hs_key = [0u8; 32]; c_hs_key.copy_from_slice(&c_hs_key_vec[..32]);
        let mut c_hs_iv = [0u8; 12]; c_hs_iv.copy_from_slice(&c_hs_iv_vec[..12]);
        let mut s_hs_key = [0u8; 32]; s_hs_key.copy_from_slice(&s_hs_key_vec[..32]);
        let mut s_hs_iv = [0u8; 12]; s_hs_iv.copy_from_slice(&s_hs_iv_vec[..12]);

        let client_cipher = TlsRecordCipher::new(&c_hs_key, &c_hs_iv);
        let server_cipher = TlsRecordCipher::new(&s_hs_key, &s_hs_iv);

        (client_cipher, server_cipher, handshake_secret, c_hs)
    }

    pub fn verify_server_finished(&mut self, server_finished_data: &[u8], s_hs: &[u8; 32]) -> Result<(), &'static str> {
        let digest_pre_fin = self.transcript.clone().finalize();
        let s_fin_key_vec = hkdf_expand_label(s_hs, "finished", b"", 32);
        let mut s_fin_key = [0u8; 32];
        s_fin_key.copy_from_slice(&s_fin_key_vec[..32]);

        let expected_verify = hmac_sha256(&s_fin_key, &digest_pre_fin);

        if server_finished_data.len() != 32 {
            return Err("Server Finished verify data length is not 32");
        }

        let mut diff = 0u8;
        for i in 0..32 {
            diff |= server_finished_data[i] ^ expected_verify[i];
        }

        if diff != 0 {
            return Err("Server Finished verify data mismatch");
        }

        // Add server Finished handshake message to transcript
        let mut fin_msg = Vec::new();
        fin_msg.push(0x14); // Finished
        fin_msg.extend_from_slice(&[0x00, 0x00, 0x20]);
        fin_msg.extend_from_slice(server_finished_data);
        self.transcript.update(&fin_msg);

        Ok(())
    }

    pub fn build_client_finished(
        &mut self,
        c_hs: &[u8; 32],
        client_hs_cipher: &mut TlsRecordCipher,
        handshake_secret: &[u8; 32],
    ) -> (Vec<u8>, TlsRecordCipher, TlsRecordCipher) {
        let digest_post_sfin = self.transcript.clone().finalize();
        let c_fin_key_vec = hkdf_expand_label(c_hs, "finished", b"", 32);
        let mut c_fin_key = [0u8; 32];
        c_fin_key.copy_from_slice(&c_fin_key_vec[..32]);

        let c_verify_data = hmac_sha256(&c_fin_key, &digest_post_sfin);

        let mut client_fin_msg = Vec::new();
        client_fin_msg.push(0x14); // Finished
        client_fin_msg.extend_from_slice(&[0x00, 0x00, 0x20]);
        client_fin_msg.extend_from_slice(&c_verify_data);

        // Update transcript with client Finished
        self.transcript.update(&client_fin_msg);

        // Encrypt client Finished record
        let enc_fin_record = client_hs_cipher.encrypt_record(CONTENT_TYPE_HANDSHAKE, &client_fin_msg);

        // Derive Application Master Secret & Traffic Keys (transcript up to server Finished per RFC 8446 Sec 7.1)
        let empty_hash = Sha256::digest(b"");
        let derived_2 = derive_secret(handshake_secret, "derived", &empty_hash);
        let master_secret = hkdf_extract(&derived_2, &[0u8; 32]);

        let c_app = derive_secret(&master_secret, "c ap traffic", &digest_post_sfin);
        let s_app = derive_secret(&master_secret, "s ap traffic", &digest_post_sfin);

        let c_app_key_vec = hkdf_expand_label(&c_app, "key", b"", 32);
        let c_app_iv_vec = hkdf_expand_label(&c_app, "iv", b"", 12);
        let s_app_key_vec = hkdf_expand_label(&s_app, "key", b"", 32);
        let s_app_iv_vec = hkdf_expand_label(&s_app, "iv", b"", 12);

        let mut c_app_key = [0u8; 32]; c_app_key.copy_from_slice(&c_app_key_vec[..32]);
        let mut c_app_iv = [0u8; 12]; c_app_iv.copy_from_slice(&c_app_iv_vec[..12]);
        let mut s_app_key = [0u8; 32]; s_app_key.copy_from_slice(&s_app_key_vec[..32]);
        let mut s_app_iv = [0u8; 12]; s_app_iv.copy_from_slice(&s_app_iv_vec[..12]);

        let client_app_cipher = TlsRecordCipher::new(&c_app_key, &c_app_iv);
        let server_app_cipher = TlsRecordCipher::new(&s_app_key, &s_app_iv);

        (enc_fin_record, client_app_cipher, server_app_cipher)
    }
}
