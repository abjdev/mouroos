use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use crate::net::ipv4::Ipv4Addr;
use crate::net::tcp::{TcpConnection, TcpState, TCP_FLAG_ACK, TCP_FLAG_FIN, TCP_FLAG_PSH, TCP_FLAG_SYN};
use crate::net::{poll, STACK};

pub struct TcpStream {
    pub local_port: u16,
    pub remote_ip: Ipv4Addr,
    pub remote_port: u16,
    pub is_closed: bool,
}

impl TcpStream {
    pub fn connect(remote_ip: Ipv4Addr, remote_port: u16) -> Result<Self, &'static str> {
        let local_port = {
            let mut stack = STACK.lock();
            let stack = stack.as_mut().ok_or("Network stack not initialized")?;
            let port = stack.allocate_ephemeral_port();

            let mut conn = TcpConnection::new(stack.ip, port, remote_ip, remote_port);
            conn.state = TcpState::SynSent;
            conn.local_seq = 10000;

            stack.send_tcp_segment(
                remote_ip,
                port,
                remote_port,
                conn.local_seq,
                0,
                TCP_FLAG_SYN,
                &[],
            );
            conn.local_seq = conn.local_seq.wrapping_add(1);

            stack.tcp_conns.insert((port, remote_ip, remote_port), conn);
            port
        };

        // Wait for handshake to complete (timeout ~4 seconds)
        let start_tick = crate::interrupts::TICKS.load(Ordering::Relaxed);
        let mut loop_count = 0usize;
        loop {
            poll();
            {
                let stack = STACK.lock();
                if let Some(stack) = stack.as_ref() {
                    if let Some(conn) = stack.tcp_conns.get(&(local_port, remote_ip, remote_port)) {
                        if conn.state == TcpState::Established {
                            return Ok(TcpStream {
                                local_port,
                                remote_ip,
                                remote_port,
                                is_closed: false,
                            });
                        } else if conn.state == TcpState::Closed {
                            return Err("Connection reset by peer");
                        }
                    }
                }
            }

            let now = crate::interrupts::TICKS.load(Ordering::Relaxed);
            loop_count += 1;
            // 400 ticks = 4 seconds at 100Hz, or fallback 4000 loops
            if now.wrapping_sub(start_tick) >= 400 || loop_count >= 4000 {
                break;
            }

            for _ in 0..5000 {
                core::hint::spin_loop();
            }
        }

        // Cleanup on timeout
        if let Some(stack) = STACK.lock().as_mut() {
            stack.tcp_conns.remove(&(local_port, remote_ip, remote_port));
        }

        Err("Connection timed out")
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        if self.is_closed {
            return Err("Socket closed");
        }

        let mut stack = STACK.lock();
        let stack = stack.as_mut().ok_or("Network stack not initialized")?;
        let conn = stack
            .tcp_conns
            .get_mut(&(self.local_port, self.remote_ip, self.remote_port))
            .ok_or("Connection not found")?;

        if conn.state != TcpState::Established && conn.state != TcpState::CloseWait {
            return Err("Connection not established");
        }

        let seq = conn.local_seq;
        let ack = conn.remote_seq;
        conn.local_seq = conn.local_seq.wrapping_add(data.len() as u32);

        stack.send_tcp_segment(
            self.remote_ip,
            self.local_port,
            self.remote_port,
            seq,
            ack,
            TCP_FLAG_ACK | TCP_FLAG_PSH,
            data,
        );

        Ok(data.len())
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if buf.is_empty() {
            return Ok(0);
        }

        let start_tick = crate::interrupts::TICKS.load(Ordering::Relaxed);
        let mut loop_count = 0usize;

        loop {
            poll();
            {
                let mut stack = STACK.lock();
                if let Some(stack) = stack.as_mut() {
                    if let Some(conn) = stack.tcp_conns.get_mut(&(self.local_port, self.remote_ip, self.remote_port)) {
                        if !conn.rx_buf.is_empty() {
                            let n = core::cmp::min(buf.len(), conn.rx_buf.len());
                            buf[..n].copy_from_slice(&conn.rx_buf[..n]);
                            conn.rx_buf.drain(..n);
                            return Ok(n);
                        }

                        if conn.state == TcpState::CloseWait || conn.state == TcpState::Closed {
                            return Ok(0); // EOF
                        }
                    } else {
                        return Ok(0);
                    }
                }
            }

            let now = crate::interrupts::TICKS.load(Ordering::Relaxed);
            loop_count += 1;
            // 400 ticks = 4 seconds at 100Hz, or fallback 4000 loops
            if now.wrapping_sub(start_tick) >= 400 || loop_count >= 4000 {
                break;
            }

            for _ in 0..5000 {
                core::hint::spin_loop();
            }
        }

        // Check whether remote closed or connection timed out
        let is_eof = {
            let stack = STACK.lock();
            if let Some(stack) = stack.as_ref() {
                if let Some(conn) = stack.tcp_conns.get(&(self.local_port, self.remote_ip, self.remote_port)) {
                    conn.state == TcpState::CloseWait || conn.state == TcpState::Closed
                } else {
                    true
                }
            } else {
                true
            }
        };

        if is_eof {
            Ok(0)
        } else {
            Err("Connection timed out waiting for data")
        }
    }

    pub fn read_to_end(&mut self, max_bytes: usize) -> Result<Vec<u8>, &'static str> {
        let mut out = Vec::new();
        let mut chunk = [0u8; 1024];

        loop {
            match self.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    out.extend_from_slice(&chunk[..n]);
                    if out.len() >= max_bytes {
                        break;
                    }
                }
                Err(e) => {
                    if !out.is_empty() {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(out)
    }

    pub fn close(&mut self) {
        if self.is_closed {
            return;
        }
        self.is_closed = true;

        let mut stack = STACK.lock();
        if let Some(stack) = stack.as_mut() {
            if let Some(conn) = stack.tcp_conns.get_mut(&(self.local_port, self.remote_ip, self.remote_port)) {
                if conn.state == TcpState::Established || conn.state == TcpState::CloseWait {
                    let seq = conn.local_seq;
                    let ack = conn.remote_seq;
                    conn.local_seq = conn.local_seq.wrapping_add(1);
                    conn.state = TcpState::FinWait1;

                    stack.send_tcp_segment(
                        self.remote_ip,
                        self.local_port,
                        self.remote_port,
                        seq,
                        ack,
                        TCP_FLAG_FIN | TCP_FLAG_ACK,
                        &[],
                    );
                }
            }
        }
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn decode_chunked_body(raw: &str) -> String {
    let mut out = String::new();
    let mut rem = raw.trim_start();
    while !rem.is_empty() {
        let line_end = match rem.find("\r\n").or_else(|| rem.find('\n')) {
            Some(i) => i,
            None => break,
        };
        let line = rem[..line_end].trim();
        let chunk_size = match usize::from_str_radix(line.split(';').next().unwrap_or(""), 16) {
            Ok(sz) => sz,
            Err(_) => {
                if out.is_empty() {
                    return String::from(raw);
                } else {
                    out.push_str(rem);
                    return out;
                }
            }
        };

        if chunk_size == 0 {
            break;
        }

        let data_start = if rem[line_end..].starts_with("\r\n") {
            line_end + 2
        } else {
            line_end + 1
        };

        if data_start + chunk_size > rem.len() {
            out.push_str(&rem[data_start..]);
            break;
        }

        out.push_str(&rem[data_start..data_start + chunk_size]);

        let next_start = data_start + chunk_size;
        rem = &rem[next_start..];
        if rem.starts_with("\r\n") {
            rem = &rem[2..];
        } else if rem.starts_with('\n') {
            rem = &rem[1..];
        }
    }
    out
}
