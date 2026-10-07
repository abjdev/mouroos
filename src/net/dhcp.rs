use alloc::vec::Vec;
use crate::net::ethernet::MacAddress;
use crate::net::ipv4::Ipv4Addr;

pub const DHCP_CLIENT_PORT: u16 = 68;
pub const DHCP_SERVER_PORT: u16 = 67;

pub const DHCP_MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];

pub const DHCP_DISCOVER: u8 = 1;
pub const DHCP_OFFER: u8 = 2;
pub const DHCP_REQUEST: u8 = 3;
pub const DHCP_ACK: u8 = 5;

pub const OPT_SUBNET_MASK: u8 = 1;
pub const OPT_ROUTER: u8 = 3;
pub const OPT_DNS: u8 = 6;
pub const OPT_REQUESTED_IP: u8 = 50;
pub const OPT_MSG_TYPE: u8 = 53;
pub const OPT_SERVER_ID: u8 = 54;
pub const OPT_PARAM_REQ_LIST: u8 = 55;
pub const OPT_END: u8 = 255;

#[derive(Clone, Debug)]
pub struct DhcpConfig {
    pub ip: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub dns_server: Ipv4Addr,
    pub server_id: Ipv4Addr,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DhcpState {
    Init,
    Selecting,
    Requesting,
    Bound,
    Failed,
}

pub struct DhcpClient {
    pub state: DhcpState,
    pub xid: u32,
    pub offered_ip: Ipv4Addr,
    pub server_id: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub dns_server: Ipv4Addr,
    pub retries: u8,
    pub timer_ticks: u64,
}

impl DhcpClient {
    pub fn new() -> Self {
        DhcpClient {
            state: DhcpState::Init,
            xid: 0x12345678,
            offered_ip: Ipv4Addr::UNSPECIFIED,
            server_id: Ipv4Addr::UNSPECIFIED,
            subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Ipv4Addr::new(10, 0, 2, 2),
            dns_server: Ipv4Addr::new(10, 0, 2, 3),
            retries: 0,
            timer_ticks: 0,
        }
    }

    pub fn build_discover(&self, mac: MacAddress, out: &mut Vec<u8>) {
        Self::build_packet(
            1, // BOOTREQUEST
            self.xid,
            mac,
            Ipv4Addr::UNSPECIFIED,
            &[
                (OPT_MSG_TYPE, &[DHCP_DISCOVER]),
                (OPT_PARAM_REQ_LIST, &[OPT_SUBNET_MASK, OPT_ROUTER, OPT_DNS]),
            ],
            out,
        );
    }

    pub fn build_request(&self, mac: MacAddress, out: &mut Vec<u8>) {
        Self::build_packet(
            1, // BOOTREQUEST
            self.xid,
            mac,
            Ipv4Addr::UNSPECIFIED,
            &[
                (OPT_MSG_TYPE, &[DHCP_REQUEST]),
                (OPT_REQUESTED_IP, &self.offered_ip.0),
                (OPT_SERVER_ID, &self.server_id.0),
                (OPT_PARAM_REQ_LIST, &[OPT_SUBNET_MASK, OPT_ROUTER, OPT_DNS]),
            ],
            out,
        );
    }

    fn build_packet(
        op: u8,
        xid: u32,
        mac: MacAddress,
        ciaddr: Ipv4Addr,
        options: &[(u8, &[u8])],
        out: &mut Vec<u8>,
    ) {
        out.clear();
        out.push(op);
        out.push(1); // htype: Ethernet
        out.push(6); // hlen
        out.push(0); // hops
        out.extend_from_slice(&xid.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // secs
        out.extend_from_slice(&0x8000u16.to_be_bytes()); // flags: broadcast
        out.extend_from_slice(&ciaddr.0); // ciaddr
        out.extend_from_slice(&[0u8; 4]); // yiaddr
        out.extend_from_slice(&[0u8; 4]); // siaddr
        out.extend_from_slice(&[0u8; 4]); // giaddr

        // chaddr (16 bytes)
        out.extend_from_slice(&mac.0);
        for _ in 0..10 {
            out.push(0);
        }

        // sname (64 bytes)
        for _ in 0..64 {
            out.push(0);
        }

        // file (128 bytes)
        for _ in 0..128 {
            out.push(0);
        }

        // Magic cookie
        out.extend_from_slice(&DHCP_MAGIC_COOKIE);

        // Options
        for &(tag, val) in options {
            out.push(tag);
            out.push(val.len() as u8);
            out.extend_from_slice(val);
        }
        out.push(OPT_END);

        // Pad to at least 300 bytes (standard DHCP min size)
        while out.len() < 300 {
            out.push(0);
        }
    }

    pub fn handle_packet(&mut self, payload: &[u8]) -> Option<DhcpConfig> {
        if payload.len() < 240 {
            return None;
        }

        let op = payload[0];
        if op != 2 {
            // Must be BOOTREPLY
            return None;
        }

        let xid = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
        if xid != self.xid {
            return None;
        }

        let yiaddr = Ipv4Addr::new(payload[16], payload[17], payload[18], payload[19]);

        // Check magic cookie
        if &payload[236..240] != &DHCP_MAGIC_COOKIE {
            return None;
        }

        let mut msg_type = None;
        let mut server_id = None;
        let mut subnet_mask = None;
        let mut router = None;
        let mut dns = None;

        let mut idx = 240;
        while idx < payload.len() {
            let opt = payload[idx];
            if opt == OPT_END {
                break;
            }
            if opt == 0 {
                // Pad option
                idx += 1;
                continue;
            }
            if idx + 1 >= payload.len() {
                break;
            }
            let len = payload[idx + 1] as usize;
            idx += 2;
            if idx + len > payload.len() {
                break;
            }
            let opt_val = &payload[idx..idx + len];
            idx += len;

            match opt {
                OPT_MSG_TYPE if len >= 1 => {
                    msg_type = Some(opt_val[0]);
                }
                OPT_SERVER_ID if len >= 4 => {
                    server_id = Some(Ipv4Addr::new(opt_val[0], opt_val[1], opt_val[2], opt_val[3]));
                }
                OPT_SUBNET_MASK if len >= 4 => {
                    subnet_mask = Some(Ipv4Addr::new(opt_val[0], opt_val[1], opt_val[2], opt_val[3]));
                }
                OPT_ROUTER if len >= 4 => {
                    router = Some(Ipv4Addr::new(opt_val[0], opt_val[1], opt_val[2], opt_val[3]));
                }
                OPT_DNS if len >= 4 => {
                    dns = Some(Ipv4Addr::new(opt_val[0], opt_val[1], opt_val[2], opt_val[3]));
                }
                _ => {}
            }
        }

        match (self.state, msg_type) {
            (DhcpState::Selecting, Some(DHCP_OFFER)) => {
                self.offered_ip = yiaddr;
                if let Some(sid) = server_id {
                    self.server_id = sid;
                }
                if let Some(mask) = subnet_mask {
                    self.subnet_mask = mask;
                }
                if let Some(gw) = router {
                    self.gateway = gw;
                }
                if let Some(d) = dns {
                    self.dns_server = d;
                }
                self.state = DhcpState::Requesting;
                None
            }
            (DhcpState::Requesting, Some(DHCP_ACK)) => {
                self.state = DhcpState::Bound;
                if let Some(mask) = subnet_mask {
                    self.subnet_mask = mask;
                }
                if let Some(gw) = router {
                    self.gateway = gw;
                }
                if let Some(d) = dns {
                    self.dns_server = d;
                }
                Some(DhcpConfig {
                    ip: yiaddr,
                    subnet_mask: self.subnet_mask,
                    gateway: self.gateway,
                    dns_server: self.dns_server,
                    server_id: self.server_id,
                })
            }
            _ => None,
        }
    }
}
