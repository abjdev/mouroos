#[derive(Clone, Copy)]
struct Fe([u64; 5]);

const MASK51: u64 = (1 << 51) - 1;

impl Fe {
    const ZERO: Fe = Fe([0, 0, 0, 0, 0]);
    const ONE: Fe = Fe([1, 0, 0, 0, 0]);

    fn from_bytes(bytes: &[u8; 32]) -> Self {
        let load64 = |start: usize| -> u64 {
            let mut buf = [0u8; 8];
            let end = core::cmp::min(start + 8, 32);
            buf[..end - start].copy_from_slice(&bytes[start..end]);
            u64::from_le_bytes(buf)
        };

        let f0 = load64(0) & MASK51;
        let f1 = (load64(6) >> 3) & MASK51;
        let f2 = (load64(12) >> 6) & MASK51;
        let f3 = (load64(19) >> 1) & MASK51;
        let f4 = (load64(25) >> 4) & ((1 << 51) - 1);

        Fe([f0, f1, f2, f3, f4])
    }

    fn to_bytes(&self) -> [u8; 32] {
        let mut t = *self;
        t.carry();
        t.carry();

        // Fully reduce modulo 2^255 - 19
        let mut q = (t.0[0] + 19) >> 51;
        q = (t.0[1] + q) >> 51;
        q = (t.0[2] + q) >> 51;
        q = (t.0[3] + q) >> 51;
        q = (t.0[4] + q) >> 51;

        t.0[0] += 19 * q;
        let c0 = t.0[0] >> 51; t.0[0] &= MASK51;
        t.0[1] += c0;
        let c1 = t.0[1] >> 51; t.0[1] &= MASK51;
        t.0[2] += c1;
        let c2 = t.0[2] >> 51; t.0[2] &= MASK51;
        t.0[3] += c2;
        let c3 = t.0[3] >> 51; t.0[3] &= MASK51;
        t.0[4] += c3;
        t.0[4] &= MASK51;

        let mut out = [0u8; 32];
        let w0 = t.0[0] | (t.0[1] << 51);
        let w1 = (t.0[1] >> 13) | (t.0[2] << 38);
        let w2 = (t.0[2] >> 26) | (t.0[3] << 25);
        let w3 = (t.0[3] >> 39) | (t.0[4] << 12);

        out[0..8].copy_from_slice(&w0.to_le_bytes());
        out[8..16].copy_from_slice(&w1.to_le_bytes());
        out[16..24].copy_from_slice(&w2.to_le_bytes());
        out[24..32].copy_from_slice(&w3.to_le_bytes());
        out
    }

    fn carry(&mut self) {
        let c0 = self.0[0] >> 51; self.0[0] &= MASK51;
        self.0[1] += c0;
        let c1 = self.0[1] >> 51; self.0[1] &= MASK51;
        self.0[2] += c1;
        let c2 = self.0[2] >> 51; self.0[2] &= MASK51;
        self.0[3] += c2;
        let c3 = self.0[3] >> 51; self.0[3] &= MASK51;
        self.0[4] += c3;
        let c4 = self.0[4] >> 51; self.0[4] &= MASK51;
        self.0[0] += c4 * 19;
    }

    fn add(&self, rhs: &Self) -> Self {
        Fe([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
            self.0[4] + rhs.0[4],
        ])
    }

    fn sub(&self, rhs: &Self) -> Self {
        // Add 2 * p (with limbs scaled) to avoid negative numbers
        const P0: u64 = MASK51 - 18;
        const P1: u64 = MASK51;
        Fe([
            self.0[0] + (P0 * 2) - rhs.0[0],
            self.0[1] + (P1 * 2) - rhs.0[1],
            self.0[2] + (P1 * 2) - rhs.0[2],
            self.0[3] + (P1 * 2) - rhs.0[3],
            self.0[4] + (P1 * 2) - rhs.0[4],
        ])
    }

    fn mul(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        let a0 = a[0] as u128; let a1 = a[1] as u128; let a2 = a[2] as u128; let a3 = a[3] as u128; let a4 = a[4] as u128;
        let b0 = b[0] as u128; let b1 = b[1] as u128; let b2 = b[2] as u128; let b3 = b[3] as u128; let b4 = b[4] as u128;

        let b1_19 = b1 * 19;
        let b2_19 = b2 * 19;
        let b3_19 = b3 * 19;
        let b4_19 = b4 * 19;

        let mut r0 = a0 * b0 + a1 * b4_19 + a2 * b3_19 + a3 * b2_19 + a4 * b1_19;
        let mut r1 = a0 * b1 + a1 * b0 + a2 * b4_19 + a3 * b3_19 + a4 * b2_19;
        let mut r2 = a0 * b2 + a1 * b1 + a2 * b0 + a3 * b4_19 + a4 * b3_19;
        let mut r3 = a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0 + a4 * b4_19;
        let mut r4 = a0 * b4 + a1 * b3 + a2 * b2 + a3 * b1 + a4 * b0;

        let c0 = (r0 >> 51) as u64; r0 &= MASK51 as u128;
        r1 += c0 as u128;
        let c1 = (r1 >> 51) as u64; r1 &= MASK51 as u128;
        r2 += c1 as u128;
        let c2 = (r2 >> 51) as u64; r2 &= MASK51 as u128;
        r3 += c2 as u128;
        let c3 = (r3 >> 51) as u64; r3 &= MASK51 as u128;
        r4 += c3 as u128;
        let c4 = (r4 >> 51) as u64; r4 &= MASK51 as u128;
        r0 += (c4 as u128) * 19;
        let c0_2 = (r0 >> 51) as u64; r0 &= MASK51 as u128;
        r1 += c0_2 as u128;

        Fe([r0 as u64, r1 as u64, r2 as u64, r3 as u64, r4 as u64])
    }

    fn sqr(&self) -> Self {
        self.mul(self)
    }

    fn mul_small(&self, s: u64) -> Self {
        let s_u128 = s as u128;
        let mut r0 = (self.0[0] as u128) * s_u128;
        let mut r1 = (self.0[1] as u128) * s_u128;
        let mut r2 = (self.0[2] as u128) * s_u128;
        let mut r3 = (self.0[3] as u128) * s_u128;
        let mut r4 = (self.0[4] as u128) * s_u128;

        let c0 = (r0 >> 51) as u64; r0 &= MASK51 as u128;
        r1 += c0 as u128;
        let c1 = (r1 >> 51) as u64; r1 &= MASK51 as u128;
        r2 += c1 as u128;
        let c2 = (r2 >> 51) as u64; r2 &= MASK51 as u128;
        r3 += c2 as u128;
        let c3 = (r3 >> 51) as u64; r3 &= MASK51 as u128;
        r4 += c3 as u128;
        let c4 = (r4 >> 51) as u64; r4 &= MASK51 as u128;
        r0 += (c4 as u128) * 19;
        let c0_2 = (r0 >> 51) as u64; r0 &= MASK51 as u128;
        r1 += c0_2 as u128;

        Fe([r0 as u64, r1 as u64, r2 as u64, r3 as u64, r4 as u64])
    }

    fn invert(&self) -> Self {
        // Compute self^(2^255 - 21) via standard addition chain
        let t0 = self.sqr();
        let t1 = t0.sqr().sqr();
        let t2 = self.mul(&t1);
        let t3 = t0.mul(&t2);
        let t4 = t3.sqr();
        let t5 = t2.mul(&t4);
        let mut t6 = t5;
        for _ in 0..5 { t6 = t6.sqr(); }
        let t7 = t6.mul(&t5);
        let mut t8 = t7;
        for _ in 0..10 { t8 = t8.sqr(); }
        let t9 = t8.mul(&t7);
        let mut t10 = t9;
        for _ in 0..20 { t10 = t10.sqr(); }
        let t11 = t10.mul(&t9);
        let mut t12 = t11;
        for _ in 0..10 { t12 = t12.sqr(); }
        let t13 = t12.mul(&t7);
        let mut t14 = t13;
        for _ in 0..50 { t14 = t14.sqr(); }
        let t15 = t14.mul(&t13);
        let mut t16 = t15;
        for _ in 0..100 { t16 = t16.sqr(); }
        let t17 = t16.mul(&t15);
        let mut t18 = t17;
        for _ in 0..50 { t18 = t18.sqr(); }
        let t19 = t18.mul(&t13);
        let mut t20 = t19;
        for _ in 0..5 { t20 = t20.sqr(); }
        t20.mul(&t3)
    }

    fn cswap(b: u8, p: &mut Fe, q: &mut Fe) {
        let mask = if b != 0 { u64::MAX } else { 0 };
        for i in 0..5 {
            let diff = mask & (p.0[i] ^ q.0[i]);
            p.0[i] ^= diff;
            q.0[i] ^= diff;
        }
    }
}

pub fn x25519(k: &[u8; 32], u: &[u8; 32]) -> [u8; 32] {
    let mut e = *k;
    e[0] &= 248;
    e[31] &= 127;
    e[31] |= 64;

    let x1 = Fe::from_bytes(u);
    let mut x2 = Fe::ONE;
    let mut z2 = Fe::ZERO;
    let mut x3 = x1;
    let mut z3 = Fe::ONE;

    let mut swap = 0u8;

    for t in (0..255).rev() {
        let bit = (e[t / 8] >> (t % 8)) & 1;
        swap ^= bit;
        Fe::cswap(swap, &mut x2, &mut x3);
        Fe::cswap(swap, &mut z2, &mut z3);
        swap = bit;

        let a = x2.add(&z2);
        let aa = a.sqr();
        let b = x2.sub(&z2);
        let bb = b.sqr();
        let e_sub = aa.sub(&bb);
        let c = x3.add(&z3);
        let d = x3.sub(&z3);
        let da = d.mul(&a);
        let cb = c.mul(&b);

        let x3_new = da.add(&cb).sqr();
        let z3_new = x1.mul(&da.sub(&cb).sqr());

        let x2_new = aa.mul(&bb);
        let z2_new = e_sub.mul(&aa.add(&e_sub.mul_small(121665)));

        x2 = x2_new;
        z2 = z2_new;
        x3 = x3_new;
        z3 = z3_new;
    }

    Fe::cswap(swap, &mut x2, &mut x3);
    Fe::cswap(swap, &mut z2, &mut z3);

    let z2_inv = z2.invert();
    let res = x2.mul(&z2_inv);
    res.to_bytes()
}

pub fn generate_keypair() -> ([u8; 32], [u8; 32]) {
    // Generate private key using TSC clock and PIT ticks
    let mut priv_key = [0u8; 32];
    let ticks = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
    let mut state = ticks.wrapping_mul(6364136223846793005).wrapping_add(1);

    // Read CPU timestamp counter (RDTSC) for high-entropy hardware jitter
    let rdtsc: u64 = unsafe {
        let low: u32;
        let high: u32;
        core::arch::asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack));
        ((high as u64) << 32) | (low as u64)
    };
    state ^= rdtsc;

    for i in 0..4 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        priv_key[i * 8..(i + 1) * 8].copy_from_slice(&state.to_le_bytes());
    }

    // Clamp private key according to RFC 7748
    priv_key[0] &= 248;
    priv_key[31] &= 127;
    priv_key[31] |= 64;

    // Standard base point is 9
    let mut base_point = [0u8; 32];
    base_point[0] = 9;

    let pub_key = x25519(&priv_key, &base_point);
    (priv_key, pub_key)
}

#[test_case]
fn test_x25519_rfc7748() {
    let alice_private: [u8; 32] = [
        0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2,
        0x66, 0x45, 0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5,
        0x1d, 0xb9, 0x2c, 0x2a,
    ];
    let mut base_point = [0u8; 32];
    base_point[0] = 9;

    let alice_public = x25519(&alice_private, &base_point);
    let expected_alice_public: [u8; 32] = [
        0x85, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54, 0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e,
        0xf7, 0x5a, 0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4, 0xeb, 0xa4, 0xa9, 0x8e,
        0xaa, 0x9b, 0x4e, 0x6a,
    ];
    assert_eq!(alice_public, expected_alice_public);

    let bob_private: [u8; 32] = [
        0x5d, 0xab, 0x08, 0x7e, 0x62, 0x4a, 0x8a, 0x4b, 0x79, 0xe1, 0x7f, 0x8b, 0x83, 0x80,
        0x0e, 0xe6, 0x6f, 0x3b, 0xb1, 0x29, 0x26, 0x18, 0xb6, 0xfd, 0x1c, 0x2f, 0x8b, 0x27,
        0xff, 0x88, 0xe0, 0xeb,
    ];
    let bob_public = x25519(&bob_private, &base_point);
    let expected_bob_public: [u8; 32] = [
        0xde, 0x9e, 0xdb, 0x7d, 0x7b, 0x7d, 0xc1, 0xb4, 0xd3, 0x5b, 0x61, 0xc2, 0xec, 0xe4,
        0x35, 0x37, 0x3f, 0x83, 0x43, 0xc8, 0x5b, 0x78, 0x67, 0x4d, 0xad, 0xfc, 0x7e, 0x14,
        0x6f, 0x88, 0x2b, 0x4f,
    ];
    assert_eq!(bob_public, expected_bob_public);

    let shared_from_alice = x25519(&alice_private, &bob_public);
    let shared_from_bob = x25519(&bob_private, &alice_public);
    let expected_shared: [u8; 32] = [
        0x4a, 0x5d, 0x9d, 0x5b, 0xa4, 0xce, 0x2d, 0xe1, 0x72, 0x8e, 0x3b, 0xf4, 0x80, 0x35,
        0x0f, 0x25, 0xe0, 0x7e, 0x21, 0xc9, 0x47, 0xd1, 0x9e, 0x33, 0x76, 0xf0, 0x9b, 0x3c,
        0x1e, 0x16, 0x17, 0x42,
    ];
    assert_eq!(shared_from_alice, expected_shared);
    assert_eq!(shared_from_bob, expected_shared);
}
