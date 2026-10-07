use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::net::ethernet::MacAddress;
use crate::net::ipv4::Ipv4Addr;

pub const ARP_OP_REQUEST: u16 = 1;
pub const ARP_OP_REPLY: u16 = 2;

#[derive(Clone, Debug)]
pub struct ArpPacket {
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hardware_size: u8,
    pub protocol_size: u8,
    pub opcode: u16,
    pub sender_mac: MacAddress,
    pub sender_ip: Ipv4Addr,
    pub target_mac: MacAddress,
    pub target_ip: Ipv4Addr,
}

impl ArpPacket {
    pub fn parse(raw: &[u8]) -> Option<Self> {
        if raw.len() < 28 {
            return None;
        }

        let hardware_type = u16::from_be_bytes([raw[0], raw[1]]);
        let protocol_type = u16::from_be_bytes([raw[2], raw[3]]);
        let hardware_size = raw[4];
        let protocol_size = raw[5];
        let opcode = u16::from_be_bytes([raw[6], raw[7]]);

        let mut smac = [0u8; 6];
        let mut sip = [0u8; 4];
        let mut tmac = [0u8; 6];
        let mut tip = [0u8; 4];

        smac.copy_from_slice(&raw[8..14]);
        sip.copy_from_slice(&raw[14..18]);
        tmac.copy_from_slice(&raw[18..24]);
        tip.copy_from_slice(&raw[24..28]);

        Some(ArpPacket {
            hardware_type,
            protocol_type,
            hardware_size,
            protocol_size,
            opcode,
            sender_mac: MacAddress(smac),
            sender_ip: Ipv4Addr(sip),
            target_mac: MacAddress(tmac),
            target_ip: Ipv4Addr(tip),
        })
    }

    pub fn build(
        opcode: u16,
        sender_mac: MacAddress,
        sender_ip: Ipv4Addr,
        target_mac: MacAddress,
        target_ip: Ipv4Addr,
        out: &mut Vec<u8>,
    ) {
        out.clear();
        out.extend_from_slice(&1u16.to_be_bytes());     // Hardware: Ethernet
        out.extend_from_slice(&0x0800u16.to_be_bytes()); // Protocol: IPv4
        out.push(6); // HW size
        out.push(4); // Proto size
        out.extend_from_slice(&opcode.to_be_bytes());
        out.extend_from_slice(&sender_mac.0);
        out.extend_from_slice(&sender_ip.0);
        out.extend_from_slice(&target_mac.0);
        out.extend_from_slice(&target_ip.0);
    }
}

pub struct ArpTable {
    entries: BTreeMap<Ipv4Addr, MacAddress>,
}

impl ArpTable {
    pub const fn new() -> Self {
        ArpTable {
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, ip: Ipv4Addr, mac: MacAddress) {
        self.entries.insert(ip, mac);
    }

    pub fn lookup(&self, ip: &Ipv4Addr) -> Option<MacAddress> {
        self.entries.get(ip).copied()
    }
}
