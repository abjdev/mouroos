use alloc::vec::Vec;
use crate::net::ipv4::{checksum, Ipv4Addr, PROTO_UDP};

#[derive(Clone, Debug)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dest_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub fn parse(raw: &[u8]) -> Option<(Self, &[u8])> {
        if raw.len() < 8 {
            return None;
        }

        let src_port = u16::from_be_bytes([raw[0], raw[1]]);
        let dest_port = u16::from_be_bytes([raw[2], raw[3]]);
        let length = u16::from_be_bytes([raw[4], raw[5]]) as usize;
        let checksum = u16::from_be_bytes([raw[6], raw[7]]);

        if raw.len() < length || length < 8 {
            return None;
        }

        let header = UdpHeader {
            src_port,
            dest_port,
            length: length as u16,
            checksum,
        };

        let payload = &raw[8..length];
        Some((header, payload))
    }

    pub fn build(
        src_ip: Ipv4Addr,
        dest_ip: Ipv4Addr,
        src_port: u16,
        dest_port: u16,
        payload: &[u8],
        out: &mut Vec<u8>,
    ) {
        let length = (8 + payload.len()) as u16;
        out.clear();
        out.extend_from_slice(&src_port.to_be_bytes());
        out.extend_from_slice(&dest_port.to_be_bytes());
        out.extend_from_slice(&length.to_be_bytes());
        out.push(0); // Checksum placeholder
        out.push(0);
        out.extend_from_slice(payload);

        // Pseudo-header for UDP checksum
        let mut pseudo = Vec::with_capacity(12 + out.len());
        pseudo.extend_from_slice(&src_ip.0);
        pseudo.extend_from_slice(&dest_ip.0);
        pseudo.push(0);
        pseudo.push(PROTO_UDP);
        pseudo.extend_from_slice(&length.to_be_bytes());
        pseudo.extend_from_slice(out);

        let mut cksum = checksum(&pseudo);
        if cksum == 0 {
            cksum = 0xFFFF;
        }
        out[6..8].copy_from_slice(&cksum.to_be_bytes());
    }
}
