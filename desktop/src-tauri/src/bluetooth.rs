use crate::a3959::{
    A3959State, build_state_request_packet, parse_a3959_state_packet,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredBtDevice {
    pub name: String,
    pub mac_address: String,
    pub bt_addr: u64,
    pub connected: bool,
    pub is_r50i_nc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketDispatchResult {
    pub transmitted_over_rfcomm: bool,
    pub packet_hex: String,
    pub state: A3959State,
}

pub fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn format_mac(bt_addr: u64) -> String {
    let b = bt_addr.to_be_bytes();
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        b[2], b[3], b[4], b[5], b[6], b[7]
    )
}

pub fn is_soundcore_r50i_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("r50i")
        || lower.contains("p30i")
        || lower.contains("a3959")
        || lower.contains("soundcore")
}

#[cfg(windows)]
pub fn scan_bluetooth_devices() -> Vec<DiscoveredBtDevice> {
    use windows::Win32::Devices::Bluetooth::{
        BLUETOOTH_DEVICE_INFO, BLUETOOTH_DEVICE_SEARCH_PARAMS, BluetoothFindDeviceClose,
        BluetoothFindFirstDevice, BluetoothFindNextDevice,
    };
    use windows::Win32::Foundation::HANDLE;
    use windows::core::BOOL;

    let mut devices = Vec::new();
    unsafe {
        let search_params = BLUETOOTH_DEVICE_SEARCH_PARAMS {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
            fReturnAuthenticated: BOOL(1),
            fReturnRemembered: BOOL(1),
            fReturnUnknown: BOOL(0),
            fReturnConnected: BOOL(1),
            fIssueInquiry: BOOL(0),
            cTimeoutMultiplier: 1,
            hRadio: HANDLE::default(),
        };
        let mut info = BLUETOOTH_DEVICE_INFO {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            ..Default::default()
        };

        if let Ok(handle) = BluetoothFindFirstDevice(&search_params, &mut info) {
            loop {
                let nul_pos = info
                    .szName
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(info.szName.len());
                let name = String::from_utf16_lossy(&info.szName[..nul_pos])
                    .trim()
                    .to_string();
                let addr = info.Address.Anonymous.ullLong;
                if addr != 0 {
                    let display_name = if name.is_empty() {
                        format!("Bluetooth Audio ({})", format_mac(addr))
                    } else {
                        name
                    };
                    devices.push(DiscoveredBtDevice {
                        is_r50i_nc: is_soundcore_r50i_name(&display_name),
                        name: display_name,
                        mac_address: format_mac(addr),
                        bt_addr: addr,
                        connected: info.fConnected.as_bool(),
                    });
                }
                info = BLUETOOTH_DEVICE_INFO {
                    dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
                    ..Default::default()
                };
                if BluetoothFindNextDevice(handle, &mut info).is_err() {
                    break;
                }
            }
            let _ = BluetoothFindDeviceClose(handle);
        }
    }
    devices.sort_by_key(|d| (!d.is_r50i_nc, !d.connected));
    devices
}

#[cfg(not(windows))]
pub fn scan_bluetooth_devices() -> Vec<DiscoveredBtDevice> {
    Vec::new()
}

#[cfg(windows)]
pub fn exchange_rfcomm_packet(
    bt_addr: u64,
    outbound: &[u8],
    expect_reply: bool,
) -> Result<Option<Vec<u8>>, String> {
    use windows::Win32::Devices::Bluetooth::{AF_BTH, BTHPROTO_RFCOMM, SOCKADDR_BTH};
    use windows::Win32::Networking::WinSock::{
        SEND_RECV_FLAGS, SOCK_STREAM, SOCKADDR, SO_RCVTIMEO, SO_SNDTIMEO, SOL_SOCKET, WSADATA,
        WSAStartup, closesocket, connect, recv, send, setsockopt, socket,
    };
    use windows::core::GUID;

    const SOUNDCORE_VENDOR_UUID: GUID = GUID::from_u128(0x0cf12d31_fac3_4553_bd80_d6832e700000);
    const STANDARD_SPP_UUID: GUID = GUID::from_u128(0x00001101_0000_1000_8000_00805f9b34fb);

    unsafe {
        let mut wsa_data = WSADATA::default();
        if WSAStartup(0x0202, &mut wsa_data) != 0 {
            return Err("WSAStartup failed".to_string());
        }

        for uuid in [SOUNDCORE_VENDOR_UUID, STANDARD_SPP_UUID] {
            let Ok(sock) = socket(AF_BTH as i32, SOCK_STREAM, BTHPROTO_RFCOMM as i32) else {
                continue;
            };

            let timeout_ms: u32 = 900;
            let timeout_bytes = timeout_ms.to_ne_bytes();
            let _ = setsockopt(sock, SOL_SOCKET, SO_SNDTIMEO, Some(&timeout_bytes));
            let _ = setsockopt(sock, SOL_SOCKET, SO_RCVTIMEO, Some(&timeout_bytes));

            let addr = SOCKADDR_BTH {
                addressFamily: AF_BTH,
                btAddr: bt_addr,
                serviceClassId: uuid,
                port: 0,
            };

            let connected = connect(
                sock,
                (&addr as *const SOCKADDR_BTH).cast::<SOCKADDR>(),
                std::mem::size_of::<SOCKADDR_BTH>() as i32,
            ) == 0;

            if connected {
                let sent = send(sock, outbound, SEND_RECV_FLAGS(0));
                if sent <= 0 {
                    let _ = closesocket(sock);
                    continue;
                }
                if !expect_reply {
                    let _ = closesocket(sock);
                    return Ok(None);
                }
                let mut reply = vec![0u8; 256];
                let n = recv(sock, &mut reply, SEND_RECV_FLAGS(0));
                let _ = closesocket(sock);
                if n > 0 {
                    reply.truncate(n as usize);
                    return Ok(Some(reply));
                }
                return Ok(None);
            } else {
                let _ = closesocket(sock);
            }
        }
    }
    Err("RFCOMM socket could not reach Soundcore R50i NC (device offline or in case)".to_string())
}

#[cfg(not(windows))]
pub fn exchange_rfcomm_packet(
    _bt_addr: u64,
    _outbound: &[u8],
    _expect_reply: bool,
) -> Result<Option<Vec<u8>>, String> {
    Err("RFCOMM only supported on Windows".to_string())
}

pub fn try_query_live_r50i_state(current: &A3959State) -> Option<A3959State> {
    let devices = scan_bluetooth_devices();
    let candidate = devices
        .iter()
        .find(|d| d.is_r50i_nc && d.connected)
        .or_else(|| devices.iter().find(|d| d.is_r50i_nc))?;

    let req = build_state_request_packet();
    if let Ok(Some(reply)) = exchange_rfcomm_packet(candidate.bt_addr, &req, true) {
        if let Ok(mut parsed) = parse_a3959_state_packet(&reply) {
            parsed.device_name = candidate.name.clone();
            parsed.mac_address = candidate.mac_address.clone();
            parsed.connected = true;
            parsed.simulated = false;
            return Some(parsed);
        }
    }

    let mut fallback = current.clone();
    fallback.device_name = candidate.name.clone();
    fallback.mac_address = candidate.mac_address.clone();
    fallback.connected = candidate.connected;
    fallback.simulated = !candidate.connected;
    Some(fallback)
}
