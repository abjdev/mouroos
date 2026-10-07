use alloc::vec::Vec;

pub struct ChaCha20 {
    state: [u32; 16],
}

impl ChaCha20 {
    pub fn new(key: &[u8; 32], nonce: &[u8; 12], counter: u32) -> Self {
        let mut state = [0u32; 16];
        state[0] = 0x61707865;
        state[1] = 0x3320646e;
        state[2] = 0x79622d32;
        state[3] = 0x6b206574;

        for i in 0..8 {
            state[4 + i] = u32::from_le_bytes([
                key[i * 4],
                key[i * 4 + 1],
                key[i * 4 + 2],
                key[i * 4 + 3],
            ]);
        }

        state[12] = counter;
        for i in 0..3 {
            state[13 + i] = u32::from_le_bytes([
                nonce[i * 4],
                nonce[i * 4 + 1],
                nonce[i * 4 + 2],
                nonce[i * 4 + 3],
            ]);
        }

        ChaCha20 { state }
    }

    #[inline(always)]
    fn qr(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
        s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(16);
        s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(12);
        s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(8);
        s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(7);
    }

    pub fn block(&mut self) -> [u8; 64] {
        let mut s = self.state;
        for _ in 0..10 {
            Self::qr(&mut s, 0, 4, 8, 12);
            Self::qr(&mut s, 1, 5, 9, 13);
            Self::qr(&mut s, 2, 6, 10, 14);
            Self::qr(&mut s, 3, 7, 11, 15);
            Self::qr(&mut s, 0, 5, 10, 15);
            Self::qr(&mut s, 1, 6, 11, 12);
            Self::qr(&mut s, 2, 7, 8, 13);
            Self::qr(&mut s, 3, 4, 9, 14);
        }

        let orig = self.state;
        self.state[12] = self.state[12].wrapping_add(1);

        let mut out = [0u8; 64];
        for i in 0..16 {
            let word = s[i].wrapping_add(orig[i]);
            out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    pub fn apply_keystream(&mut self, data: &mut [u8]) {
        let mut offset = 0;
        while offset < data.len() {
            let block = self.block();
            let count = core::cmp::min(64, data.len() - offset);
            for i in 0..count {
                data[offset + i] ^= block[i];
            }
            offset += count;
        }
    }
}

pub struct Poly1305 {
    r: [u64; 5],
    r_prime: [u64; 5],
    s: [u8; 16],
    h: [u64; 5],
    buffer: [u8; 16],
    buf_len: usize,
}

impl Poly1305 {
    pub fn new(key: &[u8; 32]) -> Self {
        let mut r_bytes = [0u8; 16];
        r_bytes.copy_from_slice(&key[0..16]);
        r_bytes[3] &= 15;
        r_bytes[7] &= 15;
        r_bytes[11] &= 15;
        r_bytes[15] &= 15;
        r_bytes[4] &= 252;
        r_bytes[8] &= 252;
        r_bytes[12] &= 252;

        let r_u128 = u128::from_le_bytes(r_bytes);
        const MASK26: u64 = (1 << 26) - 1;
        let r0 = (r_u128 & (MASK26 as u128)) as u64;
        let r1 = ((r_u128 >> 26) & (MASK26 as u128)) as u64;
        let r2 = ((r_u128 >> 52) & (MASK26 as u128)) as u64;
        let r3 = ((r_u128 >> 78) & (MASK26 as u128)) as u64;
        let r4 = ((r_u128 >> 104) & (MASK26 as u128)) as u64;

        let r = [r0, r1, r2, r3, r4];
        let r_prime = [0, r1 * 5, r2 * 5, r3 * 5, r4 * 5];

        let mut s = [0u8; 16];
        s.copy_from_slice(&key[16..32]);

        Poly1305 {
            r,
            r_prime,
            s,
            h: [0; 5],
            buffer: [0u8; 16],
            buf_len: 0,
        }
    }

    fn process_block(&mut self, block: &[u8; 16], len: usize) {
        const MASK26: u64 = (1 << 26) - 1;

        let mut full = [0u8; 17];
        full[..len].copy_from_slice(&block[..len]);
        full[len] = 0x01; // Append 0x01 bit

        let mut c_bytes = [0u8; 16];
        c_bytes.copy_from_slice(&full[..16]);
        let low128 = u128::from_le_bytes(c_bytes);
        let high_byte = full[16] as u64;

        let c0 = (low128 & (MASK26 as u128)) as u64;
        let c1 = ((low128 >> 26) & (MASK26 as u128)) as u64;
        let c2 = ((low128 >> 52) & (MASK26 as u128)) as u64;
        let c3 = ((low128 >> 78) & (MASK26 as u128)) as u64;
        let c4 = (((low128 >> 104) as u64) | (high_byte << 24)) & MASK26;

        self.h[0] += c0;
        self.h[1] += c1;
        self.h[2] += c2;
        self.h[3] += c3;
        self.h[4] += c4;

        let d0 = (self.h[0] as u128) * (self.r[0] as u128)
            + (self.h[1] as u128) * (self.r_prime[4] as u128)
            + (self.h[2] as u128) * (self.r_prime[3] as u128)
            + (self.h[3] as u128) * (self.r_prime[2] as u128)
            + (self.h[4] as u128) * (self.r_prime[1] as u128);

        let d1 = (self.h[0] as u128) * (self.r[1] as u128)
            + (self.h[1] as u128) * (self.r[0] as u128)
            + (self.h[2] as u128) * (self.r_prime[4] as u128)
            + (self.h[3] as u128) * (self.r_prime[3] as u128)
            + (self.h[4] as u128) * (self.r_prime[2] as u128);

        let d2 = (self.h[0] as u128) * (self.r[2] as u128)
            + (self.h[1] as u128) * (self.r[1] as u128)
            + (self.h[2] as u128) * (self.r[0] as u128)
            + (self.h[3] as u128) * (self.r_prime[4] as u128)
            + (self.h[4] as u128) * (self.r_prime[3] as u128);

        let d3 = (self.h[0] as u128) * (self.r[3] as u128)
            + (self.h[1] as u128) * (self.r[2] as u128)
            + (self.h[2] as u128) * (self.r[1] as u128)
            + (self.h[3] as u128) * (self.r[0] as u128)
            + (self.h[4] as u128) * (self.r_prime[4] as u128);

        let d4 = (self.h[0] as u128) * (self.r[4] as u128)
            + (self.h[1] as u128) * (self.r[3] as u128)
            + (self.h[2] as u128) * (self.r[2] as u128)
            + (self.h[3] as u128) * (self.r[1] as u128)
            + (self.h[4] as u128) * (self.r[0] as u128);

        let c0 = (d0 >> 26) as u64; self.h[0] = (d0 as u64) & MASK26;
        let d1 = d1 + (c0 as u128);
        let c1 = (d1 >> 26) as u64; self.h[1] = (d1 as u64) & MASK26;
        let d2 = d2 + (c1 as u128);
        let c2 = (d2 >> 26) as u64; self.h[2] = (d2 as u64) & MASK26;
        let d3 = d3 + (c2 as u128);
        let c3 = (d3 >> 26) as u64; self.h[3] = (d3 as u64) & MASK26;
        let d4 = d4 + (c3 as u128);
        let c4 = (d4 >> 26) as u64; self.h[4] = (d4 as u64) & MASK26;

        self.h[0] += c4 * 5;
        let c0_2 = self.h[0] >> 26; self.h[0] &= MASK26;
        self.h[1] += c0_2;
    }

    pub fn update(&mut self, mut data: &[u8]) {
        if self.buf_len > 0 {
            let needed = 16 - self.buf_len;
            if data.len() >= needed {
                self.buffer[self.buf_len..16].copy_from_slice(&data[..needed]);
                let block = self.buffer;
                self.process_block(&block, 16);
                self.buf_len = 0;
                data = &data[needed..];
            } else {
                self.buffer[self.buf_len..self.buf_len + data.len()].copy_from_slice(data);
                self.buf_len += data.len();
                return;
            }
        }

        while data.len() >= 16 {
            let mut block = [0u8; 16];
            block.copy_from_slice(&data[..16]);
            self.process_block(&block, 16);
            data = &data[16..];
        }

        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 16] {
        if self.buf_len > 0 {
            let mut block = [0u8; 16];
            block[..self.buf_len].copy_from_slice(&self.buffer[..self.buf_len]);
            let len = self.buf_len;
            self.process_block(&block, len);
        }

        const MASK26: u64 = (1 << 26) - 1;

        let mut c0 = self.h[0] >> 26; self.h[0] &= MASK26;
        self.h[1] += c0;
        let c1 = self.h[1] >> 26; self.h[1] &= MASK26;
        self.h[2] += c1;
        let c2 = self.h[2] >> 26; self.h[2] &= MASK26;
        self.h[3] += c2;
        let c3 = self.h[3] >> 26; self.h[3] &= MASK26;
        self.h[4] += c3;
        let c4 = self.h[4] >> 26; self.h[4] &= MASK26;
        self.h[0] += c4 * 5;
        c0 = self.h[0] >> 26; self.h[0] &= MASK26;
        self.h[1] += c0;

        let mut h_low = (self.h[0] as u128)
            | ((self.h[1] as u128) << 26)
            | ((self.h[2] as u128) << 52)
            | ((self.h[3] as u128) << 78)
            | (((self.h[4] as u128) & 0xffffff) << 104);
        let h_high = (self.h[4] >> 24) as u8;

        if h_high > 3 || (h_high == 3 && h_low >= (u128::MAX - 4)) {
            h_low = h_low.wrapping_add(5);
        }

        let s_val = u128::from_le_bytes(self.s);
        let tag_val = h_low.wrapping_add(s_val);
        tag_val.to_le_bytes()
    }
}

pub struct ChaCha20Poly1305 {
    key: [u8; 32],
}

impl ChaCha20Poly1305 {
    pub fn new(key: &[u8; 32]) -> Self {
        ChaCha20Poly1305 { key: *key }
    }

    pub fn encrypt(&self, nonce: &[u8; 12], plaintext: &[u8], aad: &[u8]) -> (Vec<u8>, [u8; 16]) {
        let mut cipher = ChaCha20::new(&self.key, nonce, 0);
        let poly_key_block = cipher.block();
        let mut poly_key = [0u8; 32];
        poly_key.copy_from_slice(&poly_key_block[..32]);

        let mut ciphertext = plaintext.to_vec();
        cipher.apply_keystream(&mut ciphertext);

        let mut poly = Poly1305::new(&poly_key);
        poly.update(aad);
        if aad.len() % 16 != 0 {
            let pad = [0u8; 16];
            poly.update(&pad[..16 - (aad.len() % 16)]);
        }

        poly.update(&ciphertext);
        if ciphertext.len() % 16 != 0 {
            let pad = [0u8; 16];
            poly.update(&pad[..16 - (ciphertext.len() % 16)]);
        }

        poly.update(&(aad.len() as u64).to_le_bytes());
        poly.update(&(ciphertext.len() as u64).to_le_bytes());

        let tag = poly.finalize();
        (ciphertext, tag)
    }

    pub fn decrypt(&self, nonce: &[u8; 12], ciphertext: &[u8], tag: &[u8; 16], aad: &[u8]) -> Result<Vec<u8>, &'static str> {
        let mut cipher = ChaCha20::new(&self.key, nonce, 0);
        let poly_key_block = cipher.block();
        let mut poly_key = [0u8; 32];
        poly_key.copy_from_slice(&poly_key_block[..32]);

        let mut poly = Poly1305::new(&poly_key);
        poly.update(aad);
        if aad.len() % 16 != 0 {
            let pad = [0u8; 16];
            poly.update(&pad[..16 - (aad.len() % 16)]);
        }

        poly.update(ciphertext);
        if ciphertext.len() % 16 != 0 {
            let pad = [0u8; 16];
            poly.update(&pad[..16 - (ciphertext.len() % 16)]);
        }

        poly.update(&(aad.len() as u64).to_le_bytes());
        poly.update(&(ciphertext.len() as u64).to_le_bytes());

        let expected_tag = poly.finalize();

        let mut diff = 0u8;
        for i in 0..16 {
            diff |= tag[i] ^ expected_tag[i];
        }

        if diff != 0 {
            return Err("Poly1305 authentication tag mismatch");
        }

        let mut plaintext = ciphertext.to_vec();
        cipher.apply_keystream(&mut plaintext);
        Ok(plaintext)
    }
}

#[test_case]
fn test_aead_rfc8439() {
    let key: [u8; 32] = [
        0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d,
        0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b,
        0x9c, 0x9d, 0x9e, 0x9f,
    ];
    let nonce: [u8; 12] = [
        0x07, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47,
    ];
    let aad: [u8; 12] = [
        0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
    ];
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

    let cipher = ChaCha20Poly1305::new(&key);
    let (ct, tag) = cipher.encrypt(&nonce, plaintext, &aad);

    let expected_tag: [u8; 16] = [
        0x1a, 0xe1, 0x0b, 0x59, 0x4f, 0x09, 0xe2, 0x6a, 0x7e, 0x90, 0x2e, 0xcb, 0xd0, 0x60,
        0x06, 0x91,
    ];
    assert_eq!(tag, expected_tag);

    let decrypted = cipher.decrypt(&nonce, &ct, &tag, &aad).unwrap();
    assert_eq!(decrypted, plaintext);
}
