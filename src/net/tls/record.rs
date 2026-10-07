use alloc::vec::Vec;
use crate::net::tls::chacha20poly1305::ChaCha20Poly1305;

pub const CONTENT_TYPE_CHANGE_CIPHER_SPEC: u8 = 0x14;
pub const CONTENT_TYPE_ALERT: u8 = 0x15;
pub const CONTENT_TYPE_HANDSHAKE: u8 = 0x16;
pub const CONTENT_TYPE_APPLICATION_DATA: u8 = 0x17;

pub struct TlsRecordCipher {
    cipher: ChaCha20Poly1305,
    iv: [u8; 12],
    pub seq_num: u64,
}

impl TlsRecordCipher {
    pub fn new(key: &[u8; 32], iv: &[u8; 12]) -> Self {
        TlsRecordCipher {
            cipher: ChaCha20Poly1305::new(key),
            iv: *iv,
            seq_num: 0,
        }
    }

    fn compute_nonce(&self) -> [u8; 12] {
        let mut nonce = self.iv;
        let seq_bytes = self.seq_num.to_be_bytes();
        // XOR sequence number into lowest 8 bytes of 12-byte IV
        for i in 0..8 {
            nonce[4 + i] ^= seq_bytes[i];
        }
        nonce
    }

    pub fn encrypt_record(&mut self, content_type: u8, data: &[u8]) -> Vec<u8> {
        let nonce = self.compute_nonce();
        self.seq_num += 1;

        // Plaintext = data + inner_content_type
        let mut inner_plaintext = Vec::with_capacity(data.len() + 1);
        inner_plaintext.extend_from_slice(data);
        inner_plaintext.push(content_type);

        let ciphertext_len = (inner_plaintext.len() + 16) as u16;
        let aad = [
            CONTENT_TYPE_APPLICATION_DATA,
            0x03,
            0x03,
            (ciphertext_len >> 8) as u8,
            (ciphertext_len & 0xff) as u8,
        ];

        let (ciphertext, tag) = self.cipher.encrypt(&nonce, &inner_plaintext, &aad);

        let mut record = Vec::with_capacity(5 + ciphertext.len() + 16);
        record.extend_from_slice(&aad);
        record.extend_from_slice(&ciphertext);
        record.extend_from_slice(&tag);
        record
    }

    pub fn decrypt_record(&mut self, header: &[u8; 5], ciphertext_with_tag: &[u8]) -> Result<(u8, Vec<u8>), &'static str> {
        if ciphertext_with_tag.len() < 16 {
            return Err("Record too short for tag");
        }

        let nonce = self.compute_nonce();
        self.seq_num += 1;

        let ct_len = ciphertext_with_tag.len() - 16;
        let ct = &ciphertext_with_tag[..ct_len];
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&ciphertext_with_tag[ct_len..]);

        let inner_plaintext = self.cipher.decrypt(&nonce, ct, &tag, header)?;

        // Strip trailing zero padding
        let mut idx = inner_plaintext.len();
        while idx > 0 && inner_plaintext[idx - 1] == 0 {
            idx -= 1;
        }

        if idx == 0 {
            return Err("Malformed TLS 1.3 inner plaintext");
        }

        let inner_type = inner_plaintext[idx - 1];
        let content = inner_plaintext[..idx - 1].to_vec();

        Ok((inner_type, content))
    }
}
