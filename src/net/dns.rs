use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use crate::net::ipv4::Ipv4Addr;

pub const DNS_PORT: u16 = 53;
pub const TYPE_A: u16 = 1;
pub const CLASS_IN: u16 = 1;

pub struct DnsResolver {
    cache: BTreeMap<String, Ipv4Addr>,
    next_id: u16,
}

impl DnsResolver {
    pub const fn new() -> Self {
        DnsResolver {
            cache: BTreeMap::new(),
            next_id: 0x4321,
        }
    }

    pub fn get_cached(&self, host: &str) -> Option<Ipv4Addr> {
        self.cache.get(host).copied()
    }

    pub fn insert_cache(&mut self, host: &str, ip: Ipv4Addr) {
        self.cache.insert(String::from(host), ip);
    }

    pub fn build_query(&mut self, host: &str, out: &mut Vec<u8>) -> u16 {
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;

        out.clear();
        // Header
        out.extend_from_slice(&id.to_be_bytes());
        out.extend_from_slice(&0x0100u16.to_be_bytes()); // RD = 1 (Recursion Desired)
        out.extend_from_slice(&1u16.to_be_bytes());      // QDCOUNT = 1
        out.extend_from_slice(&0u16.to_be_bytes());      // ANCOUNT = 0
        out.extend_from_slice(&0u16.to_be_bytes());      // NSCOUNT = 0
        out.extend_from_slice(&0u16.to_be_bytes());      // ARCOUNT = 0

        // Question: QNAME
        for part in host.split('.') {
            if part.is_empty() {
                continue;
            }
            out.push(part.len() as u8);
            out.extend_from_slice(part.as_bytes());
        }
        out.push(0); // Zero terminator

        // QTYPE = 1 (A), QCLASS = 1 (IN)
        out.extend_from_slice(&TYPE_A.to_be_bytes());
        out.extend_from_slice(&CLASS_IN.to_be_bytes());

        id
    }

    pub fn parse_response(&mut self, host: &str, raw: &[u8], query_id: u16) -> Option<Ipv4Addr> {
        if raw.len() < 12 {
            return None;
        }

        let id = u16::from_be_bytes([raw[0], raw[1]]);
        if id != query_id {
            return None;
        }

        let flags = u16::from_be_bytes([raw[2], raw[3]]);
        if (flags & 0x8000) == 0 {
            // Not a response
            return None;
        }
        let rcode = flags & 0x000F;
        if rcode != 0 {
            // Error response
            return None;
        }

        let qdcount = u16::from_be_bytes([raw[4], raw[5]]) as usize;
        let ancount = u16::from_be_bytes([raw[6], raw[7]]) as usize;

        if ancount == 0 {
            return None;
        }

        let mut offset = 12;

        // Skip question section
        for _ in 0..qdcount {
            offset = skip_name(raw, offset)?;
            offset += 4; // Skip QTYPE and QCLASS
            if offset > raw.len() {
                return None;
            }
        }

        // Parse answers
        for _ in 0..ancount {
            offset = skip_name(raw, offset)?;
            if offset + 10 > raw.len() {
                return None;
            }
            let atype = u16::from_be_bytes([raw[offset], raw[offset + 1]]);
            let aclass = u16::from_be_bytes([raw[offset + 2], raw[offset + 3]]);
            // offset + 4..8 is TTL
            let rdlength = u16::from_be_bytes([raw[offset + 8], raw[offset + 9]]) as usize;
            offset += 10;

            if offset + rdlength > raw.len() {
                return None;
            }

            if atype == TYPE_A && aclass == CLASS_IN && rdlength == 4 {
                let ip = Ipv4Addr::new(
                    raw[offset],
                    raw[offset + 1],
                    raw[offset + 2],
                    raw[offset + 3],
                );
                self.insert_cache(host, ip);
                return Some(ip);
            }

            offset += rdlength;
        }

        None
    }
}

fn skip_name(data: &[u8], mut offset: usize) -> Option<usize> {
    while offset < data.len() {
        let len = data[offset];
        if len == 0 {
            return Some(offset + 1);
        } else if (len & 0xC0) == 0xC0 {
            // Compressed pointer (2 bytes)
            return Some(offset + 2);
        } else {
            offset += 1 + (len as usize);
        }
    }
    None
}
