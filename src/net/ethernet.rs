use alloc::vec::Vec;
use core::fmt;

pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16 = 0x0806;

#[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: MacAddress = MacAddress([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    pub const ZERO: MacAddress = MacAddress([0, 0, 0, 0, 0, 0]);

    pub fn new(bytes: [u8; 6]) -> Self {
        MacAddress(bytes)
    }

    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }

    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return None;
        }
        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16).ok()?;
        }
        Some(MacAddress(bytes))
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub struct EthernetFrame<'a> {
    pub dest: MacAddress,
    pub src: MacAddress,
    pub ethertype: u16,
    pub payload: &'a [u8],
}

impl<'a> EthernetFrame<'a> {
    pub fn parse(raw: &'a [u8]) -> Option<Self> {
        if raw.len() < 14 {
            return None;
        }

        let mut dest = [0u8; 6];
        let mut src = [0u8; 6];
        dest.copy_from_slice(&raw[0..6]);
        src.copy_from_slice(&raw[6..12]);
        let ethertype = u16::from_be_bytes([raw[12], raw[13]]);

        Some(EthernetFrame {
            dest: MacAddress(dest),
            src: MacAddress(src),
            ethertype,
            payload: &raw[14..],
        })
    }

    pub fn build(dest: MacAddress, src: MacAddress, ethertype: u16, payload: &[u8], out: &mut Vec<u8>) {
        out.clear();
        out.extend_from_slice(&dest.0);
        out.extend_from_slice(&src.0);
        out.extend_from_slice(&ethertype.to_be_bytes());
        out.extend_from_slice(payload);

        // Ethernet frames must be at least 60 bytes (excluding 4-byte FCS)
        while out.len() < 60 {
            out.push(0);
        }
    }
}
