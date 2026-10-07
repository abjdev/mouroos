use alloc::vec::Vec;
use crate::net::ipv4::{checksum, Ipv4Addr, PROTO_TCP};

// TCP Flags
pub const TCP_FLAG_FIN: u8 = 0x01;
pub const TCP_FLAG_SYN: u8 = 0x02;
pub const TCP_FLAG_RST: u8 = 0x04;
pub const TCP_FLAG_PSH: u8 = 0x08;
pub const TCP_FLAG_ACK: u8 = 0x10;
pub const TCP_FLAG_URG: u8 = 0x20;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TcpState {
    Closed,
    SynSent,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

#[derive(Clone, Debug)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dest_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset: u8,
    pub flags: u8,
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
}

impl TcpHeader {
    pub fn parse(raw: &[u8]) -> Option<(Self, &[u8])> {
        if raw.len() < 20 {
            return None;
        }

        let src_port = u16::from_be_bytes([raw[0], raw[1]]);
        let dest_port = u16::from_be_bytes([raw[2], raw[3]]);
        let seq_num = u32::from_be_bytes([raw[4], raw[5], raw[6], raw[7]]);
        let ack_num = u32::from_be_bytes([raw[8], raw[9], raw[10], raw[11]]);
        let data_offset = (raw[12] >> 4) * 4;
        let flags = raw[13];
        let window_size = u16::from_be_bytes([raw[14], raw[15]]);
        let checksum = u16::from_be_bytes([raw[16], raw[17]]);
        let urgent_ptr = u16::from_be_bytes([raw[18], raw[19]]);

        if raw.len() < data_offset as usize || (data_offset as usize) < 20 {
            return None;
        }

        let header = TcpHeader {
            src_port,
            dest_port,
            seq_num,
            ack_num,
            data_offset,
            flags,
            window_size,
            checksum,
            urgent_ptr,
        };

        let payload = &raw[data_offset as usize..];
        Some((header, payload))
    }

    pub fn build(
        src_ip: Ipv4Addr,
        dest_ip: Ipv4Addr,
        src_port: u16,
        dest_port: u16,
        seq_num: u32,
        ack_num: u32,
        flags: u8,
        window_size: u16,
        payload: &[u8],
        out: &mut Vec<u8>,
    ) {
        let total_len = (20 + payload.len()) as u16;
        out.clear();
        out.extend_from_slice(&src_port.to_be_bytes());
        out.extend_from_slice(&dest_port.to_be_bytes());
        out.extend_from_slice(&seq_num.to_be_bytes());
        out.extend_from_slice(&ack_num.to_be_bytes());
        out.push(0x50); // Data offset: 5 * 4 = 20 bytes
        out.push(flags);
        out.extend_from_slice(&window_size.to_be_bytes());
        out.push(0); // Checksum placeholder
        out.push(0);
        out.extend_from_slice(&0u16.to_be_bytes()); // Urgent pointer
        out.extend_from_slice(payload);

        // Pseudo-header
        let mut pseudo = Vec::with_capacity(12 + out.len());
        pseudo.extend_from_slice(&src_ip.0);
        pseudo.extend_from_slice(&dest_ip.0);
        pseudo.push(0);
        pseudo.push(PROTO_TCP);
        pseudo.extend_from_slice(&total_len.to_be_bytes());
        pseudo.extend_from_slice(out);

        let cksum = checksum(&pseudo);
        out[16..18].copy_from_slice(&cksum.to_be_bytes());
    }
}

pub struct TcpConnection {
    pub local_ip: Ipv4Addr,
    pub local_port: u16,
    pub remote_ip: Ipv4Addr,
    pub remote_port: u16,
    pub state: TcpState,
    pub local_seq: u32,
    pub remote_seq: u32,
    pub rx_buf: Vec<u8>,
    pub pending_tx: Vec<u8>,
    pub last_activity_ticks: u64,
}

impl TcpConnection {
    pub fn new(local_ip: Ipv4Addr, local_port: u16, remote_ip: Ipv4Addr, remote_port: u16) -> Self {
        TcpConnection {
            local_ip,
            local_port,
            remote_ip,
            remote_port,
            state: TcpState::Closed,
            local_seq: 1000,
            remote_seq: 0,
            rx_buf: Vec::new(),
            pending_tx: Vec::new(),
            last_activity_ticks: 0,
        }
    }
}
