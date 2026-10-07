use alloc::vec::Vec;
use core::fmt;

pub const PROTO_ICMP: u8 = 1;
pub const PROTO_TCP: u8 = 6;
pub const PROTO_UDP: u8 = 17;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Ipv4Addr(pub [u8; 4]);

impl Ipv4Addr {
    pub const UNSPECIFIED: Ipv4Addr = Ipv4Addr([0, 0, 0, 0]);
    pub const BROADCAST: Ipv4Addr = Ipv4Addr([255, 255, 255, 255]);
    pub const LOOPBACK: Ipv4Addr = Ipv4Addr([127, 0, 0, 1]);

    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Ipv4Addr([a, b, c, d])
    }

    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return None;
        }
        let mut octets = [0u8; 4];
        for (i, part) in parts.iter().enumerate() {
            octets[i] = part.parse::<u8>().ok()?;
        }
        Some(Ipv4Addr(octets))
    }

    pub fn is_unspecified(&self) -> bool {
        *self == Self::UNSPECIFIED
    }

    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl fmt::Debug for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub fn checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;
    while i + 1 < data.len() {
        let word = u16::from_be_bytes([data[i], data[i + 1]]);
        sum += word as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

#[derive(Clone, Debug)]
pub struct Ipv4Header {
    pub src: Ipv4Addr,
    pub dest: Ipv4Addr,
    pub protocol: u8,
    pub ttl: u8,
    pub identification: u16,
    pub total_length: u16,
}

impl Ipv4Header {
    pub fn parse<'a>(raw: &'a [u8]) -> Option<(Self, &'a [u8])> {
        if raw.len() < 20 {
            return None;
        }

        let version_ihl = raw[0];
        let version = version_ihl >> 4;
        let ihl = (version_ihl & 0x0F) as usize * 4;

        if version != 4 || ihl < 20 || raw.len() < ihl {
            return None;
        }

        let total_length = u16::from_be_bytes([raw[2], raw[3]]) as usize;
        if raw.len() < total_length || total_length < ihl {
            return None;
        }

        let identification = u16::from_be_bytes([raw[4], raw[5]]);
        let ttl = raw[8];
        let protocol = raw[9];

        let mut src = [0u8; 4];
        let mut dest = [0u8; 4];
        src.copy_from_slice(&raw[12..16]);
        dest.copy_from_slice(&raw[16..20]);

        let header = Ipv4Header {
            src: Ipv4Addr(src),
            dest: Ipv4Addr(dest),
            protocol,
            ttl,
            identification,
            total_length: total_length as u16,
        };

        let payload = &raw[ihl..total_length];
        Some((header, payload))
    }

    pub fn build(
        src: Ipv4Addr,
        dest: Ipv4Addr,
        protocol: u8,
        id: u16,
        payload: &[u8],
        out: &mut Vec<u8>,
    ) {
        let total_len = (20 + payload.len()) as u16;
        let mut header = [0u8; 20];

        header[0] = 0x45; // Version 4, IHL 5 (20 bytes)
        header[1] = 0x00; // DSCP / ECN
        header[2..4].copy_from_slice(&total_len.to_be_bytes());
        header[4..6].copy_from_slice(&id.to_be_bytes());
        header[6..8].copy_from_slice(&0x4000u16.to_be_bytes()); // Don't Fragment flag
        header[8] = 64;   // TTL = 64
        header[9] = protocol;
        header[10..12].copy_from_slice(&0u16.to_be_bytes()); // Checksum placeholder
        header[12..16].copy_from_slice(&src.0);
        header[16..20].copy_from_slice(&dest.0);

        let cksum = checksum(&header);
        header[10..12].copy_from_slice(&cksum.to_be_bytes());

        out.clear();
        out.extend_from_slice(&header);
        out.extend_from_slice(payload);
    }
}
