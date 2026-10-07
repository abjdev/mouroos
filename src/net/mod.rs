pub mod arp;
pub mod dhcp;
pub mod dns;
pub mod ethernet;
pub mod icmp;
pub mod ipv4;
pub mod socket;
pub mod tcp;
pub mod udp;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use spin::Mutex;

use crate::drivers::rtl8139;
use crate::net::arp::{ArpPacket, ArpTable, ARP_OP_REPLY, ARP_OP_REQUEST};
use crate::net::dhcp::{DhcpClient, DhcpConfig, DhcpState, DHCP_CLIENT_PORT, DHCP_SERVER_PORT};
use crate::net::dns::{DnsResolver, DNS_PORT};
use crate::net::ethernet::{EthernetFrame, MacAddress, ETHERTYPE_ARP, ETHERTYPE_IPV4};
use crate::net::icmp::{IcmpEchoPacket, ICMP_TYPE_ECHO_REPLY, ICMP_TYPE_ECHO_REQUEST};
use crate::net::ipv4::{Ipv4Addr, Ipv4Header, PROTO_ICMP, PROTO_TCP, PROTO_UDP};
use crate::net::tcp::{TcpConnection, TcpHeader, TcpState, TCP_FLAG_ACK, TCP_FLAG_FIN, TCP_FLAG_RST, TCP_FLAG_SYN};
use crate::net::udp::UdpHeader;

#[derive(Clone, Debug)]
pub struct NetStats {
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct PingReply {
    pub seq: u16,
    pub sender: Ipv4Addr,
    pub ticks: u64,
}

pub struct NetworkStack {
    pub mac: MacAddress,
    pub ip: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub dns_server: Ipv4Addr,
    pub arp_table: ArpTable,
    pub dns_resolver: DnsResolver,
    pub dhcp_client: DhcpClient,
    pub tcp_conns: BTreeMap<(u16, Ipv4Addr, u16), TcpConnection>,
    pub ping_replies: Vec<PingReply>,
    pub stats: NetStats,
    pub next_port: u16,
    pub last_tick: u64,
}

pub static STACK: Mutex<Option<NetworkStack>> = Mutex::new(None);

impl NetworkStack {
    pub fn new(mac: MacAddress) -> Self {
        let mut arp_table = ArpTable::new();
        // Pre-populate router and DNS server MACs for QEMU SLIRP (52:55:0A:00:02:02)
        arp_table.insert(Ipv4Addr::new(10, 0, 2, 2), MacAddress::new([0x52, 0x55, 0x0A, 0x00, 0x02, 0x02]));
        arp_table.insert(Ipv4Addr::new(10, 0, 2, 3), MacAddress::new([0x52, 0x55, 0x0A, 0x00, 0x02, 0x03]));

        NetworkStack {
            mac,
            ip: Ipv4Addr::new(10, 0, 2, 15), // Standard QEMU default
            subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Ipv4Addr::new(10, 0, 2, 2),
            dns_server: Ipv4Addr::new(10, 0, 2, 3),
            arp_table,
            dns_resolver: DnsResolver::new(),
            dhcp_client: DhcpClient::new(),
            tcp_conns: BTreeMap::new(),
            ping_replies: Vec::new(),
            stats: NetStats {
                rx_packets: 0,
                tx_packets: 0,
                rx_bytes: 0,
                tx_bytes: 0,
            },
            next_port: 49152,
            last_tick: 0,
        }
    }

    pub fn allocate_ephemeral_port(&mut self) -> u16 {
        let p = self.next_port;
        self.next_port = if self.next_port >= 65530 { 49152 } else { self.next_port + 1 };
        p
    }

    pub fn send_ethernet(&mut self, dest: MacAddress, ethertype: u16, payload: &[u8]) -> bool {
        let mut frame = Vec::new();
        EthernetFrame::build(dest, self.mac, ethertype, payload, &mut frame);
        let len = frame.len();
        let ok = rtl8139::send(&frame);
        if ok {
            self.stats.tx_packets += 1;
            self.stats.tx_bytes += len as u64;
        }
        ok
    }

    pub fn send_ipv4(&mut self, dest_ip: Ipv4Addr, protocol: u8, payload: &[u8]) -> bool {
        let mut ip_packet = Vec::new();
        let id = (self.stats.tx_packets & 0xFFFF) as u16;
        Ipv4Header::build(self.ip, dest_ip, protocol, id, payload, &mut ip_packet);

        // Determine destination MAC address
        let target_ip = if dest_ip == Ipv4Addr::BROADCAST {
            dest_ip
        } else if (dest_ip.0[0] & self.subnet_mask.0[0]) == (self.ip.0[0] & self.subnet_mask.0[0])
            && (dest_ip.0[1] & self.subnet_mask.0[1]) == (self.ip.0[1] & self.subnet_mask.0[1])
            && (dest_ip.0[2] & self.subnet_mask.0[2]) == (self.ip.0[2] & self.subnet_mask.0[2])
        {
            dest_ip
        } else {
            self.gateway
        };

        let dest_mac = if target_ip == Ipv4Addr::BROADCAST {
            MacAddress::BROADCAST
        } else if let Some(mac) = self.arp_table.lookup(&target_ip) {
            mac
        } else {
            // Need ARP resolution; broadcast ARP request first
            self.send_arp_request(target_ip);
            // In QEMU, router mac is 52:55:0A:00:02:02
            MacAddress([0x52, 0x55, 0x0A, 0x00, 0x02, 0x02])
        };

        self.send_ethernet(dest_mac, ETHERTYPE_IPV4, &ip_packet)
    }

    pub fn send_arp_request(&mut self, target_ip: Ipv4Addr) {
        let mut packet = Vec::new();
        ArpPacket::build(
            ARP_OP_REQUEST,
            self.mac,
            self.ip,
            MacAddress::ZERO,
            target_ip,
            &mut packet,
        );
        self.send_ethernet(MacAddress::BROADCAST, ETHERTYPE_ARP, &packet);
    }

    pub fn send_arp_reply(&mut self, target_mac: MacAddress, target_ip: Ipv4Addr) {
        let mut packet = Vec::new();
        ArpPacket::build(
            ARP_OP_REPLY,
            self.mac,
            self.ip,
            target_mac,
            target_ip,
            &mut packet,
        );
        self.send_ethernet(target_mac, ETHERTYPE_ARP, &packet);
    }

    pub fn send_udp(&mut self, dest_ip: Ipv4Addr, src_port: u16, dest_port: u16, payload: &[u8]) -> bool {
        let mut udp_buf = Vec::new();
        UdpHeader::build(self.ip, dest_ip, src_port, dest_port, payload, &mut udp_buf);
        self.send_ipv4(dest_ip, PROTO_UDP, &udp_buf)
    }

    pub fn send_tcp_segment(
        &mut self,
        dest_ip: Ipv4Addr,
        src_port: u16,
        dest_port: u16,
        seq: u32,
        ack: u32,
        flags: u8,
        payload: &[u8],
    ) -> bool {
        let mut tcp_buf = Vec::new();
        TcpHeader::build(
            self.ip,
            dest_ip,
            src_port,
            dest_port,
            seq,
            ack,
            flags,
            65535,
            payload,
            &mut tcp_buf,
        );
        self.send_ipv4(dest_ip, PROTO_TCP, &tcp_buf)
    }

    pub fn poll_incoming(&mut self) {
        let mut raw_buf = [0u8; 2048];
        while let Some(len) = rtl8139::poll(&mut raw_buf) {
            self.stats.rx_packets += 1;
            self.stats.rx_bytes += len as u64;

            let frame = match EthernetFrame::parse(&raw_buf[..len]) {
                Some(f) => f,
                None => continue,
            };

            // Check if intended for us or broadcast
            if !frame.dest.is_broadcast() && frame.dest != self.mac {
                continue;
            }

            match frame.ethertype {
                ETHERTYPE_ARP => {
                    if let Some(arp) = ArpPacket::parse(frame.payload) {
                        self.arp_table.insert(arp.sender_ip, arp.sender_mac);
                        if arp.opcode == ARP_OP_REQUEST && arp.target_ip == self.ip {
                            self.send_arp_reply(arp.sender_mac, arp.sender_ip);
                        }
                    }
                }
                ETHERTYPE_IPV4 => {
                    let (ipv4_hdr, payload) = match Ipv4Header::parse(frame.payload) {
                        Some(res) => res,
                        None => continue,
                    };

                    match ipv4_hdr.protocol {
                        PROTO_ICMP => {
                            if let Some(icmp) = IcmpEchoPacket::parse(payload) {
                                if icmp.icmp_type == ICMP_TYPE_ECHO_REQUEST && ipv4_hdr.dest == self.ip {
                                    let mut reply_buf = Vec::new();
                                    IcmpEchoPacket::build_echo_reply(icmp.id, icmp.seq, icmp.payload, &mut reply_buf);
                                    self.send_ipv4(ipv4_hdr.src, PROTO_ICMP, &reply_buf);
                                } else if icmp.icmp_type == ICMP_TYPE_ECHO_REPLY {
                                    self.ping_replies.push(PingReply {
                                        seq: icmp.seq,
                                        sender: ipv4_hdr.src,
                                        ticks: self.last_tick,
                                    });
                                }
                            }
                        }
                        PROTO_UDP => {
                            if let Some((udp_hdr, udp_payload)) = UdpHeader::parse(payload) {
                                if udp_hdr.dest_port == DHCP_CLIENT_PORT {
                                    if let Some(cfg) = self.dhcp_client.handle_packet(udp_payload) {
                                        self.apply_dhcp_config(cfg);
                                    }
                                } else if udp_hdr.src_port == DNS_PORT {
                                    // Parse DNS response
                                    self.dns_resolver.parse_response(udp_payload);
                                }
                            }
                        }
                        PROTO_TCP => {
                            if let Some((tcp_hdr, tcp_payload)) = TcpHeader::parse(payload) {
                                self.handle_incoming_tcp(ipv4_hdr.src, tcp_hdr, tcp_payload);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_incoming_tcp(&mut self, remote_ip: Ipv4Addr, hdr: TcpHeader, payload: &[u8]) {
        let key = (hdr.dest_port, remote_ip, hdr.src_port);
        let mut send_ack = false;
        let mut ack_num = 0u32;
        let mut local_seq = 0u32;

        if let Some(conn) = self.tcp_conns.get_mut(&key) {
            if (hdr.flags & TCP_FLAG_RST) != 0 {
                conn.state = TcpState::Closed;
                return;
            }

            match conn.state {
                TcpState::SynSent => {
                    if (hdr.flags & (TCP_FLAG_SYN | TCP_FLAG_ACK)) == (TCP_FLAG_SYN | TCP_FLAG_ACK) {
                        conn.remote_seq = hdr.seq_num.wrapping_add(1);
                        conn.local_seq = hdr.ack_num;
                        conn.state = TcpState::Established;
                        send_ack = true;
                        ack_num = conn.remote_seq;
                        local_seq = conn.local_seq;
                    }
                }
                TcpState::Established => {
                    if !payload.is_empty() {
                        conn.rx_buf.extend_from_slice(payload);
                        conn.remote_seq = hdr.seq_num.wrapping_add(payload.len() as u32);
                        send_ack = true;
                        ack_num = conn.remote_seq;
                        local_seq = conn.local_seq;
                    }

                    if (hdr.flags & TCP_FLAG_FIN) != 0 {
                        conn.remote_seq = hdr.seq_num.wrapping_add(if payload.is_empty() { 1 } else { payload.len() as u32 + 1 });
                        conn.state = TcpState::CloseWait;
                        send_ack = true;
                        ack_num = conn.remote_seq;
                        local_seq = conn.local_seq;
                    }
                }
                TcpState::FinWait1 => {
                    if (hdr.flags & TCP_FLAG_ACK) != 0 {
                        conn.state = TcpState::Closed;
                    }
                    if (hdr.flags & TCP_FLAG_FIN) != 0 {
                        conn.remote_seq = hdr.seq_num.wrapping_add(1);
                        conn.state = TcpState::Closed;
                        send_ack = true;
                        ack_num = conn.remote_seq;
                        local_seq = conn.local_seq;
                    }
                }
                TcpState::CloseWait => {
                    if (hdr.flags & TCP_FLAG_FIN) != 0 {
                        send_ack = true;
                        ack_num = conn.remote_seq;
                        local_seq = conn.local_seq;
                    }
                }
                TcpState::FinWait2 => {
                    if (hdr.flags & TCP_FLAG_FIN) != 0 {
                        conn.remote_seq = hdr.seq_num.wrapping_add(1);
                        conn.state = TcpState::Closed;
                        send_ack = true;
                        ack_num = conn.remote_seq;
                        local_seq = conn.local_seq;
                    }
                }
                _ => {}
            }
        }

        if send_ack {
            self.send_tcp_segment(
                remote_ip,
                hdr.dest_port,
                hdr.src_port,
                local_seq,
                ack_num,
                TCP_FLAG_ACK,
                &[],
            );
        }
    }

    pub fn apply_dhcp_config(&mut self, cfg: DhcpConfig) {
        crate::serial_println!(
            "[DHCP] Config acquired: IP={}, Netmask={}, Gateway={}, DNS={}",
            cfg.ip, cfg.subnet_mask, cfg.gateway, cfg.dns_server
        );
        self.ip = cfg.ip;
        self.subnet_mask = cfg.subnet_mask;
        self.gateway = cfg.gateway;
        self.dns_server = cfg.dns_server;
    }

    pub fn start_dhcp(&mut self) {
        self.dhcp_client.state = DhcpState::Selecting;
        let mut disc_buf = Vec::new();
        self.dhcp_client.build_discover(self.mac, &mut disc_buf);
        let mut udp_buf = Vec::new();
        UdpHeader::build(
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::BROADCAST,
            DHCP_CLIENT_PORT,
            DHCP_SERVER_PORT,
            &disc_buf,
            &mut udp_buf,
        );
        let mut ip_buf = Vec::new();
        Ipv4Header::build(
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::BROADCAST,
            PROTO_UDP,
            0x1234,
            &udp_buf,
            &mut ip_buf,
        );
        self.send_ethernet(MacAddress::BROADCAST, ETHERTYPE_IPV4, &ip_buf);
    }
}

pub fn init() {
    let mac_raw = match rtl8139::get_mac() {
        Some(m) => m,
        None => [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
    };
    let mac = MacAddress(mac_raw);
    let mut stack = NetworkStack::new(mac);
    stack.start_dhcp();
    *STACK.lock() = Some(stack);
    crate::serial_println!("[NET] Network stack initialized. MAC={}", mac);
}

pub fn poll() {
    if let Some(stack) = STACK.lock().as_mut() {
        stack.last_tick = stack.last_tick.wrapping_add(1);
        stack.poll_incoming();
    }
}

pub fn get_info() -> Option<(MacAddress, Ipv4Addr, Ipv4Addr, Ipv4Addr, Ipv4Addr, NetStats)> {
    STACK.lock().as_ref().map(|s| {
        (
            s.mac,
            s.ip,
            s.subnet_mask,
            s.gateway,
            s.dns_server,
            s.stats.clone(),
        )
    })
}

pub fn set_static_config(ip: Ipv4Addr, mask: Ipv4Addr, gw: Ipv4Addr, dns: Ipv4Addr) {
    if let Some(stack) = STACK.lock().as_mut() {
        stack.ip = ip;
        stack.subnet_mask = mask;
        stack.gateway = gw;
        stack.dns_server = dns;
    }
}

pub fn resolve_hostname(host: &str) -> Option<Ipv4Addr> {
    if let Some(ip) = Ipv4Addr::parse(host) {
        return Some(ip);
    }

    // 1. Check cache first
    if let Some(stack) = STACK.lock().as_ref() {
        if let Some(ip) = stack.dns_resolver.get_cached(host) {
            return Some(ip);
        }
    }

    // 2. Build DNS query and send UDP packet; release STACK.lock() immediately
    let host_str = String::from(host);
    let mut query = Vec::new();
    {
        let mut guard = STACK.lock();
        if let Some(stack) = guard.as_mut() {
            let _qid = stack.dns_resolver.build_query(host, &mut query);
            let dns_server = stack.dns_server;
            let src_port = stack.allocate_ephemeral_port();
            stack.send_udp(dns_server, src_port, DNS_PORT, &query);
        } else {
            return None;
        }
    }

    // 3. Poll for response WITHOUT holding STACK lock during poll()
    let start_tick = crate::interrupts::TICKS.load(Ordering::Relaxed);
    let mut loop_count = 0usize;
    loop {
        poll();
        if let Some(stack) = STACK.lock().as_ref() {
            if let Some(ip) = stack.dns_resolver.get_cached(&host_str) {
                return Some(ip);
            }
        }
        let now = crate::interrupts::TICKS.load(Ordering::Relaxed);
        loop_count += 1;
        // 300 ticks = 3 seconds at 100Hz, or fallback 3000 loops
        if now.wrapping_sub(start_tick) >= 300 || loop_count >= 3000 {
            break;
        }
        for _ in 0..5000 {
            core::hint::spin_loop();
        }
    }

    None
}
