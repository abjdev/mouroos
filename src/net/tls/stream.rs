use alloc::vec::Vec;
use crate::net::ipv4::Ipv4Addr;
use crate::net::socket::TcpStream;
use crate::net::tls::handshake::HandshakeBuilder;
use crate::net::tls::record::{
    TlsRecordCipher, CONTENT_TYPE_ALERT, CONTENT_TYPE_APPLICATION_DATA,
    CONTENT_TYPE_CHANGE_CIPHER_SPEC, CONTENT_TYPE_HANDSHAKE,
};

pub struct TlsStream {
    tcp: TcpStream,
    client_app_cipher: TlsRecordCipher,
    server_app_cipher: TlsRecordCipher,
    raw_rx_buf: Vec<u8>,
    rx_buf: Vec<u8>,
    is_closed: bool,
}

impl TlsStream {
    pub fn connect(remote_ip: Ipv4Addr, port: u16, hostname: &str) -> Result<Self, &'static str> {
        // 1. Establish TCP connection to remote host (port 443)
        let mut tcp = TcpStream::connect(remote_ip, port)?;

        // 2. Build ClientHello
        let mut builder = HandshakeBuilder::new(hostname);
        let client_hello_record = builder.build_client_hello();

        // 3. Send ClientHello over TCP
        tcp.write(&client_hello_record)?;

        // 4. Read ServerHello record(s) from TCP
        let mut sh_handshake_bytes: Option<Vec<u8>> = None;
        let mut read_buf = [0u8; 4096];

        // Read records until we find ServerHello
        let mut temp_buf = Vec::new();
        while sh_handshake_bytes.is_none() {
            let n = tcp.read(&mut read_buf)?;
            if n == 0 {
                return Err("Connection closed before ServerHello");
            }
            temp_buf.extend_from_slice(&read_buf[..n]);

            // Try to extract records from temp_buf
            while temp_buf.len() >= 5 {
                let rec_type = temp_buf[0];
                let rec_len = u16::from_be_bytes([temp_buf[3], temp_buf[4]]) as usize;
                if temp_buf.len() < 5 + rec_len {
                    break; // Wait for full record
                }

                let rec_payload = temp_buf[5..5 + rec_len].to_vec();
                temp_buf.drain(..5 + rec_len);

                if rec_type == CONTENT_TYPE_CHANGE_CIPHER_SPEC {
                    // Middlebox compatibility dummy record, ignore
                    continue;
                } else if rec_type == CONTENT_TYPE_HANDSHAKE {
                    if !rec_payload.is_empty() && rec_payload[0] == 0x02 {
                        sh_handshake_bytes = Some(rec_payload);
                        break;
                    }
                } else if rec_type == CONTENT_TYPE_ALERT {
                    return Err("Server sent TLS alert during ServerHello");
                }
            }
        }

        let sh_bytes = sh_handshake_bytes.ok_or("ServerHello not received")?;
        let sh_info = builder.parse_server_hello(&sh_bytes)?;
        crate::serial_println!("[TLS] ServerHello parsed: cipher=0x{:04x}, pub={:02x?}", sh_info.cipher_suite, &sh_info.server_pub[..4]);

        // 5. Derive Handshake secrets and initialize Handshake Ciphers
        let (mut client_hs_cipher, mut server_hs_cipher, handshake_secret, c_hs) =
            builder.compute_handshake_secrets(&sh_info.server_pub);

        // Compute s_hs for server Finished verification
        let digest_sh = builder.transcript.clone().finalize();
        let s_hs = crate::net::tls::sha256::derive_secret(&handshake_secret, "s hs traffic", &digest_sh);

        // 6. Receive encrypted Server Handshake messages:
        //    EncryptedExtensions, Certificate, CertificateVerify, Finished
        let mut server_finished_verified = false;
        let mut hs_msg_stream = Vec::new();

        while !server_finished_verified {
            // Check if temp_buf has an encrypted record (type = 0x17)
            while temp_buf.len() >= 5 {
                let rec_type = temp_buf[0];
                let rec_len = u16::from_be_bytes([temp_buf[3], temp_buf[4]]) as usize;
                if temp_buf.len() < 5 + rec_len {
                    break;
                }

                let mut header = [0u8; 5];
                header.copy_from_slice(&temp_buf[..5]);
                let ciphertext = temp_buf[5..5 + rec_len].to_vec();
                temp_buf.drain(..5 + rec_len);

                if rec_type == CONTENT_TYPE_CHANGE_CIPHER_SPEC {
                    crate::serial_println!("[TLS] Middlebox CCS received");
                    continue;
                }

                if rec_type == CONTENT_TYPE_APPLICATION_DATA {
                    let (inner_type, plaintext) = match server_hs_cipher.decrypt_record(&header, &ciphertext) {
                        Ok(res) => res,
                        Err(e) => {
                            crate::serial_println!("[TLS] Handshake decrypt failed: {}, rec_len={}, seq={}, header={:02x?}", e, rec_len, server_hs_cipher.seq_num, header);
                            return Err(e);
                        }
                    };
                    if inner_type == CONTENT_TYPE_HANDSHAKE {
                        hs_msg_stream.extend_from_slice(&plaintext);
                    } else if inner_type == CONTENT_TYPE_ALERT {
                        return Err("Server sent TLS alert during handshake");
                    }
                }
            }

            // Parse any complete handshake messages from hs_msg_stream
            while hs_msg_stream.len() >= 4 {
                let msg_type = hs_msg_stream[0];
                let msg_len = ((hs_msg_stream[1] as usize) << 16)
                    | ((hs_msg_stream[2] as usize) << 8)
                    | (hs_msg_stream[3] as usize);

                if hs_msg_stream.len() < 4 + msg_len {
                    break; // Message incomplete, wait for more records
                }

                let full_msg = hs_msg_stream[..4 + msg_len].to_vec();
                let body = &full_msg[4..];

                if msg_type == 20 {
                    // Finished message
                    builder.verify_server_finished(body, &s_hs)?;
                    server_finished_verified = true;
                    hs_msg_stream.drain(..4 + msg_len);
                    break;
                } else {
                    // EncryptedExtensions (8), Certificate (11), CertificateVerify (15)
                    builder.transcript.update(&full_msg);
                    hs_msg_stream.drain(..4 + msg_len);
                }
            }

            if !server_finished_verified {
                let n = tcp.read(&mut read_buf)?;
                if n == 0 {
                    return Err("Connection closed before server Finished");
                }
                temp_buf.extend_from_slice(&read_buf[..n]);
            }
        }

        // 7. Send middlebox ChangeCipherSpec
        let ccs = [CONTENT_TYPE_CHANGE_CIPHER_SPEC, 0x03, 0x03, 0x00, 0x01, 0x01];
        tcp.write(&ccs)?;

        // 8. Build, encrypt, and send client Finished record
        let (client_fin_record, client_app_cipher, server_app_cipher) =
            builder.build_client_finished(&c_hs, &mut client_hs_cipher, &handshake_secret);

        tcp.write(&client_fin_record)?;

        crate::serial_println!("[TLS] Handshake complete with {}! Protocol: TLS 1.3 ChaCha20-Poly1305", hostname);

        Ok(TlsStream {
            tcp,
            client_app_cipher,
            server_app_cipher,
            raw_rx_buf: temp_buf,
            rx_buf: Vec::new(),
            is_closed: false,
        })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        if self.is_closed {
            return Err("TLS stream is closed");
        }

        // Chunk large writes into 16KB records
        let mut total_sent = 0;
        let mut offset = 0;

        while offset < data.len() {
            let chunk_len = core::cmp::min(16384, data.len() - offset);
            let chunk = &data[offset..offset + chunk_len];
            let record = self.client_app_cipher.encrypt_record(CONTENT_TYPE_APPLICATION_DATA, chunk);
            self.tcp.write(&record)?;
            offset += chunk_len;
            total_sent += chunk_len;
        }

        Ok(total_sent)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if buf.is_empty() {
            return Ok(0);
        }

        // If data is already in rx_buf, return it
        if !self.rx_buf.is_empty() {
            let n = core::cmp::min(buf.len(), self.rx_buf.len());
            buf[..n].copy_from_slice(&self.rx_buf[..n]);
            self.rx_buf.drain(..n);
            return Ok(n);
        }

        if self.is_closed {
            return Ok(0);
        }

        // Read records from TCP until application data is received or EOF
        let mut raw_buf = [0u8; 4096];

        loop {
            // Process complete records in self.raw_rx_buf
            while self.raw_rx_buf.len() >= 5 {
                let rec_type = self.raw_rx_buf[0];
                let rec_len = u16::from_be_bytes([self.raw_rx_buf[3], self.raw_rx_buf[4]]) as usize;
                if self.raw_rx_buf.len() < 5 + rec_len {
                    break;
                }

                let mut header = [0u8; 5];
                header.copy_from_slice(&self.raw_rx_buf[..5]);
                let ciphertext = self.raw_rx_buf[5..5 + rec_len].to_vec();
                self.raw_rx_buf.drain(..5 + rec_len);

                if rec_type == CONTENT_TYPE_CHANGE_CIPHER_SPEC {
                    continue;
                }

                if rec_type == CONTENT_TYPE_APPLICATION_DATA {
                    let (inner_type, plaintext) = self.server_app_cipher.decrypt_record(&header, &ciphertext)?;
                    if inner_type == CONTENT_TYPE_APPLICATION_DATA {
                        self.rx_buf.extend_from_slice(&plaintext);
                    } else if inner_type == CONTENT_TYPE_ALERT {
                        self.is_closed = true;
                        break;
                    }
                    // Ignore post-handshake messages like NewSessionTicket (inner_type == 0x16)
                } else if rec_type == CONTENT_TYPE_ALERT {
                    self.is_closed = true;
                    break;
                }
            }

            if !self.rx_buf.is_empty() {
                let n = core::cmp::min(buf.len(), self.rx_buf.len());
                buf[..n].copy_from_slice(&self.rx_buf[..n]);
                self.rx_buf.drain(..n);
                return Ok(n);
            }

            if self.is_closed {
                return Ok(0);
            }

            let n = self.tcp.read(&mut raw_buf)?;
            if n == 0 {
                self.is_closed = true;
                return Ok(0);
            }
            self.raw_rx_buf.extend_from_slice(&raw_buf[..n]);
        }
    }

    pub fn read_to_end(&mut self, max_bytes: usize) -> Result<Vec<u8>, &'static str> {
        let mut out = Vec::new();
        let mut chunk = [0u8; 2048];

        loop {
            match self.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    out.extend_from_slice(&chunk[..n]);
                    if out.len() >= max_bytes {
                        break;
                    }
                }
                Err(e) => {
                    if !out.is_empty() {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(out)
    }

    pub fn close(&mut self) {
        if !self.is_closed {
            self.is_closed = true;
            // Send close_notify alert: alert level 1 (warning), description 0 (close_notify)
            let alert_data = [0x01, 0x00];
            let rec = self.client_app_cipher.encrypt_record(CONTENT_TYPE_ALERT, &alert_data);
            let _ = self.tcp.write(&rec);
            self.tcp.close();
        }
    }
}

impl Drop for TlsStream {
    fn drop(&mut self) {
        self.close();
    }
}
