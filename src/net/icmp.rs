use alloc::vec::Vec;
use crate::net::ipv4::checksum;

pub const ICMP_TYPE_ECHO_REPLY: u8 = 0;
pub const ICMP_TYPE_ECHO_REQUEST: u8 = 8;

#[derive(Clone, Debug)]
pub struct IcmpEchoPacket<'a> {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: u16,
    pub id: u16,
    pub seq: u16,
    pub payload: &'a [u8],
}

impl<'a> IcmpEchoPacket<'a> {
    pub fn parse(raw: &'a [u8]) -> Option<Self> {
        if raw.len() < 8 {
            return None;
        }

        let icmp_type = raw[0];
        let code = raw[1];
        let checksum = u16::from_be_bytes([raw[2], raw[3]]);
        let id = u16::from_be_bytes([raw[4], raw[5]]);
        let seq = u16::from_be_bytes([raw[6], raw[7]]);
        let payload = &raw[8..];

        Some(IcmpEchoPacket {
            icmp_type,
            code,
            checksum,
            id,
            seq,
            payload,
        })
    }

    pub fn build_echo_request(id: u16, seq: u16, payload: &[u8], out: &mut Vec<u8>) {
        Self::build(ICMP_TYPE_ECHO_REQUEST, 0, id, seq, payload, out);
    }

    pub fn build_echo_reply(id: u16, seq: u16, payload: &[u8], out: &mut Vec<u8>) {
        Self::build(ICMP_TYPE_ECHO_REPLY, 0, id, seq, payload, out);
    }

    pub fn build(
        icmp_type: u8,
        code: u8,
        id: u16,
        seq: u16,
        payload: &[u8],
        out: &mut Vec<u8>,
    ) {
        out.clear();
        out.push(icmp_type);
        out.push(code);
        out.push(0); // Checksum placeholder
        out.push(0);
        out.extend_from_slice(&id.to_be_bytes());
        out.extend_from_slice(&seq.to_be_bytes());
        out.extend_from_slice(payload);

        let cksum = checksum(out);
        out[2..4].copy_from_slice(&cksum.to_be_bytes());
    }
}
