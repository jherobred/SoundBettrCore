//! Talking to the R50i NC (A3959): one open RFCOMM socket, every command waits for the earbuds' reply.

use crate::a3959::calculate_checksum;
use serde::Serialize;

const FROM_DEVICE: [u8; 5] = [0x09, 0xff, 0x00, 0x00, 0x01];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "camelCase")]
pub enum CommandResult {
    Confirmed,
    NoReply,
    Failed { reason: String },
}

pub trait Hardware: Send {
    /// Sends a packet and returns the earbuds' matching reply, if any arrived in time.
    fn exchange(&mut self, packet: &[u8]) -> Result<Option<Vec<u8>>, String>;
}

/// Finds the device packet answering `sent` (same command bytes, valid checksum) inside `buf`.
pub fn find_reply<'a>(sent: &[u8], buf: &'a [u8]) -> Option<&'a [u8]> {
    if sent.len() < 7 {
        return None;
    }
    let mut i = 0;
    while i + 10 <= buf.len() {
        if buf[i..i + 5] == FROM_DEVICE {
            let len = u16::from_le_bytes([buf[i + 7], buf[i + 8]]) as usize;
            if len >= 10 && i + len <= buf.len() {
                let pkt = &buf[i..i + len];
                if pkt[5..7] == sent[5..7] && calculate_checksum(&pkt[..len - 1]) == pkt[len - 1] {
                    return Some(pkt);
                }
                i += len;
                continue;
            }
        }
        i += 1;
    }
    None
}

pub fn send(hw: &mut dyn Hardware, packet: &[u8]) -> CommandResult {
    match hw.exchange(packet) {
        Ok(Some(_)) => CommandResult::Confirmed,
        Ok(None) => CommandResult::NoReply,
        Err(reason) => CommandResult::Failed { reason },
    }
}

pub fn parse_mac(mac: &str) -> Option<u64> {
    let hex: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (hex.len() == 12).then(|| u64::from_str_radix(&hex, 16).ok()).flatten()
}

#[cfg(windows)]
pub use win::RfcommSession;

#[cfg(windows)]
mod win {
    use super::*;
    use std::time::{Duration, Instant};
    use windows::core::GUID;
    use windows::Win32::Devices::Bluetooth::{AF_BTH, BTHPROTO_RFCOMM, SOCKADDR_BTH};
    use windows::Win32::Networking::WinSock::{
        closesocket, connect, recv, send, setsockopt, socket, WSAStartup, INVALID_SOCKET, SEND_RECV_FLAGS, SOCKADDR, SOCKET,
        SOCK_STREAM, SOL_SOCKET, SO_RCVTIMEO, WSADATA,
    };

    const UUIDS: [GUID; 2] = [
        GUID::from_u128(0x0cf12d31_fac3_4553_bd80_d6832e700000),
        GUID::from_u128(0x00001101_0000_1000_8000_00805f9b34fb),
    ];

    pub struct RfcommSession {
        address: u64,
        sock: Option<SOCKET>,
    }

    impl RfcommSession {
        pub fn new(address: u64) -> Self {
            RfcommSession { address, sock: None }
        }

        fn open(&mut self) -> Result<SOCKET, String> {
            if let Some(s) = self.sock {
                return Ok(s);
            }
            unsafe {
                let mut wsa = WSADATA::default();
                WSAStartup(0x0202, &mut wsa);
                for uuid in UUIDS {
                    let Ok(s) = socket(AF_BTH as i32, SOCK_STREAM, BTHPROTO_RFCOMM as i32) else { continue };
                    if s == INVALID_SOCKET {
                        continue;
                    }
                    let addr = SOCKADDR_BTH { addressFamily: AF_BTH, btAddr: self.address, serviceClassId: uuid, port: 0 };
                    if connect(s, (&addr as *const SOCKADDR_BTH).cast::<SOCKADDR>(), size_of::<SOCKADDR_BTH>() as i32) == 0 {
                        let t = 300u32.to_ne_bytes();
                        let _ = setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, Some(&t));
                        self.sock = Some(s);
                        return Ok(s);
                    }
                    let _ = closesocket(s);
                }
            }
            Err("The earbuds did not accept a control connection. Take them out of the case and try again.".into())
        }

        fn close(&mut self) {
            if let Some(s) = self.sock.take() {
                unsafe {
                    let _ = closesocket(s);
                }
            }
        }

        fn try_exchange(&mut self, packet: &[u8]) -> Result<Option<Vec<u8>>, String> {
            let s = self.open()?;
            unsafe {
                if send(s, packet, SEND_RECV_FLAGS(0)) <= 0 {
                    return Err("send failed".into());
                }
                let mut buf = Vec::new();
                let mut chunk = [0u8; 512];
                let until = Instant::now() + Duration::from_millis(900);
                while Instant::now() < until {
                    let n = recv(s, &mut chunk, SEND_RECV_FLAGS(0));
                    if n > 0 {
                        buf.extend_from_slice(&chunk[..n as usize]);
                        if let Some(r) = find_reply(packet, &buf) {
                            return Ok(Some(r.to_vec()));
                        }
                    }
                }
                Ok(None)
            }
        }
    }

    impl Hardware for RfcommSession {
        /// One reconnect on failure, because Windows drops idle RFCOMM links.
        fn exchange(&mut self, packet: &[u8]) -> Result<Option<Vec<u8>>, String> {
            match self.try_exchange(packet) {
                Ok(r) => Ok(r),
                Err(_) => {
                    self.close();
                    self.try_exchange(packet)
                }
            }
        }
    }

    impl Drop for RfcommSession {
        fn drop(&mut self) {
            self.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a3959::{build_state_request_packet, encode_host_packet};

    fn device_packet(cmd: [u8; 2], body: &[u8]) -> Vec<u8> {
        let mut p = encode_host_packet(cmd, body);
        p[..5].copy_from_slice(&FROM_DEVICE);
        let n = p.len();
        p[n - 1] = calculate_checksum(&p[..n - 1]);
        p
    }

    #[test]
    fn reply_matches_command_bytes_and_checksum() {
        let sent = build_state_request_packet();
        let cmd = [sent[5], sent[6]];
        let good = device_packet(cmd, &[1, 2, 3]);
        let other = device_packet([0x06, 0x81], &[0]);
        let mut buf = other.clone();
        buf.extend_from_slice(&good);
        assert_eq!(find_reply(&sent, &buf), Some(&good[..]));
        assert_eq!(find_reply(&sent, &other), None);
        let mut bad = good.clone();
        *bad.last_mut().unwrap() ^= 0xff;
        assert_eq!(find_reply(&sent, &bad), None);
    }

    #[test]
    fn parses_mac() {
        assert_eq!(parse_mac("A0:E9:DB:12:34:56"), Some(0xA0E9DB123456));
        assert_eq!(parse_mac("nope"), None);
    }
}
