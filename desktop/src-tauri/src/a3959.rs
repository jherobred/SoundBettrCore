use serde::{Deserialize, Serialize};

pub const DIRECTION_HOST_TO_DEVICE: [u8; 5] = [0x08, 0xEE, 0x00, 0x00, 0x00];
pub const DIRECTION_DEVICE_TO_HOST: [u8; 5] = [0x09, 0xFF, 0x00, 0x00, 0x01];

pub const CMD_STATE_REQUEST: [u8; 2] = [0x01, 0x01];
pub const CMD_SET_TOUCH_TONE: [u8; 2] = [0x01, 0x83];
pub const CMD_SET_GAMING_MODE: [u8; 2] = [0x01, 0x87];
pub const CMD_SET_EQUALIZER_DRC: [u8; 2] = [0x02, 0x83];
pub const CMD_SET_SURROUND_SOUND: [u8; 2] = [0x02, 0x86];
pub const CMD_SET_BUTTON_ACTION: [u8; 2] = [0x04, 0x81];
pub const CMD_SET_SOUND_MODES: [u8; 2] = [0x06, 0x81];

pub const CUSTOM_EQ_PRESET_ID: u16 = 0xFEFE;
pub const BASS_UP_PRESET_ID: u16 = 0x0002;
pub const FLAT_PRESET_ID: u16 = 0x0009;
pub const SIGNATURE_PRESET_ID: u16 = 0x0000;

pub const A3959_HW_BAND_FREQS: [u32; 8] = [100, 200, 400, 800, 1600, 3200, 6400, 12800];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbientSoundMode {
    NoiseCanceling,
    Transparency,
    Normal,
}

impl AmbientSoundMode {
    pub fn to_id(self) -> u8 {
        match self {
            Self::NoiseCanceling => 0,
            Self::Transparency => 1,
            Self::Normal => 2,
        }
    }

    pub fn from_id(id: u8) -> Self {
        match id {
            0 => Self::NoiseCanceling,
            1 => Self::Transparency,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AncMode {
    Manual,
    Adaptive,
    MultiScene,
}

impl AncMode {
    pub fn to_id(self) -> u8 {
        match self {
            Self::Manual => 0,
            Self::Adaptive => 1,
            Self::MultiScene => 2,
        }
    }

    pub fn from_id(id: u8) -> Self {
        match id {
            0 => Self::Manual,
            1 => Self::Adaptive,
            _ => Self::MultiScene,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSceneAnc {
    Transport,
    Outdoor,
    Indoor,
}

impl MultiSceneAnc {
    pub fn to_id(self) -> u8 {
        match self {
            Self::Transport => 0,
            Self::Outdoor => 1,
            Self::Indoor => 2,
        }
    }

    pub fn from_id(id: u8) -> Self {
        match id {
            0 => Self::Transport,
            1 => Self::Outdoor,
            _ => Self::Indoor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A3959SoundModes {
    pub ambient_mode: AmbientSoundMode,
    pub manual_anc_level: u8,
    pub adaptive_anc_level: u8,
    pub anc_mode: AncMode,
    pub wind_noise_suppression: bool,
    pub adaptive_sensitivity: u8,
    pub multi_scene_anc: MultiSceneAnc,
}

impl Default for A3959SoundModes {
    fn default() -> Self {
        Self {
            ambient_mode: AmbientSoundMode::NoiseCanceling,
            manual_anc_level: 5,
            adaptive_anc_level: 3,
            anc_mode: AncMode::Adaptive,
            wind_noise_suppression: true,
            adaptive_sensitivity: 5,
            multi_scene_anc: MultiSceneAnc::Transport,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EarbudSide {
    Left,
    Right,
}

impl EarbudSide {
    pub fn to_id(self) -> u8 {
        match self {
            Self::Left => 0x00,
            Self::Right => 0x01,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonPressKind {
    Single,
    Double,
    Triple,
    Long,
}

impl ButtonPressKind {
    pub fn to_id(self) -> u8 {
        match self {
            Self::Double => 0x00,
            Self::Long => 0x01,
            Self::Single => 0x02,
            Self::Triple => 0x05,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonAction {
    VolumeUp,
    VolumeDown,
    PreviousSong,
    NextSong,
    AmbientSoundMode,
    VoiceAssistant,
    PlayPause,
    GameMode,
    Disabled,
}

impl ButtonAction {
    pub fn to_action_nibble(self) -> Option<u8> {
        match self {
            Self::VolumeUp => Some(0x00),
            Self::VolumeDown => Some(0x01),
            Self::PreviousSong => Some(0x02),
            Self::NextSong => Some(0x03),
            Self::AmbientSoundMode => Some(0x04),
            Self::VoiceAssistant => Some(0x05),
            Self::PlayPause => Some(0x06),
            Self::GameMode => Some(0x09),
            Self::Disabled => None,
        }
    }

    pub fn from_raw_byte(raw: u8, tws_connected: bool, is_single_or_triple: bool) -> Self {
        if raw == 0xFF {
            return Self::Disabled;
        }
        if is_single_or_triple && !tws_connected {
            return Self::Disabled;
        }
        match raw & 0x0F {
            0x00 => Self::VolumeUp,
            0x01 => Self::VolumeDown,
            0x02 => Self::PreviousSong,
            0x03 => Self::NextSong,
            0x04 => Self::AmbientSoundMode,
            0x05 => Self::VoiceAssistant,
            0x06 => Self::PlayPause,
            0x09 => Self::GameMode,
            _ => Self::Disabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A3959ButtonConfig {
    pub left_single: ButtonAction,
    pub right_single: ButtonAction,
    pub left_double: ButtonAction,
    pub right_double: ButtonAction,
    pub left_triple: ButtonAction,
    pub right_triple: ButtonAction,
    pub left_long: ButtonAction,
    pub right_long: ButtonAction,
}

impl Default for A3959ButtonConfig {
    fn default() -> Self {
        Self {
            left_single: ButtonAction::VolumeDown,
            right_single: ButtonAction::VolumeUp,
            left_double: ButtonAction::PlayPause,
            right_double: ButtonAction::NextSong,
            left_triple: ButtonAction::VoiceAssistant,
            right_triple: ButtonAction::GameMode,
            left_long: ButtonAction::AmbientSoundMode,
            right_long: ButtonAction::AmbientSoundMode,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A3959State {
    pub connected: bool,
    pub simulated: bool,
    pub device_name: String,
    pub mac_address: String,
    pub host_device: u8,
    pub tws_connected: bool,
    pub left_battery_level: u8,
    pub right_battery_level: u8,
    pub left_battery_percent: u8,
    pub right_battery_percent: u8,
    pub left_charging: bool,
    pub right_charging: bool,
    pub firmware_left: String,
    pub firmware_right: String,
    pub serial_number: String,
    pub eq_preset_id: u16,
    pub eq_gains_tenths_db: [i16; 8],
    pub drc_preview_tenths_db: [i16; 8],
    pub bass_up_enabled: bool,
    pub buttons: A3959ButtonConfig,
    pub ambient_cycle_mask: u8,
    pub sound_modes: A3959SoundModes,
    pub touch_tone: bool,
    pub dual_connection: bool,
    pub onboard_surround: bool,
    pub auto_power_off_enabled: bool,
    pub auto_power_off_index: u8,
    pub low_battery_prompt: bool,
    pub gaming_mode: Option<bool>,
}

impl Default for A3959State {
    fn default() -> Self {
        let eq_gains_tenths_db = [-25, -18, 5, 18, 32, 24, 10, 15];
        let drc_bytes = compute_a3959_drc_bytes(&eq_gains_tenths_db);
        let drc_preview_tenths_db = std::array::from_fn(|i| (drc_bytes[i] as i16) - 120);
        Self {
            connected: false,
            simulated: true,
            device_name: "Soundcore R50i NC (A3959)".to_string(),
            mac_address: "AC:12:2F:39:59:A0".to_string(),
            host_device: 1,
            tws_connected: true,
            left_battery_level: 9,
            right_battery_level: 9,
            left_battery_percent: 90,
            right_battery_percent: 90,
            left_charging: false,
            right_charging: false,
            firmware_left: "01.64".to_string(),
            firmware_right: "01.64".to_string(),
            serial_number: "3959685923BF067C".to_string(),
            eq_preset_id: CUSTOM_EQ_PRESET_ID,
            eq_gains_tenths_db,
            drc_preview_tenths_db,
            bass_up_enabled: false,
            buttons: A3959ButtonConfig::default(),
            ambient_cycle_mask: 0x07,
            sound_modes: A3959SoundModes::default(),
            touch_tone: true,
            dual_connection: true,
            onboard_surround: false,
            auto_power_off_enabled: true,
            auto_power_off_index: 1,
            low_battery_prompt: true,
            gaming_mode: Some(false),
        }
    }
}

pub fn calculate_checksum(bytes_without_checksum: &[u8]) -> u8 {
    bytes_without_checksum
        .iter()
        .fold(0u8, |acc, &b| acc.wrapping_add(b))
}

pub fn encode_host_packet(command: [u8; 2], body: &[u8]) -> Vec<u8> {
    let total_len = 5 + 2 + 2 + body.len() + 1;
    let mut pkt = Vec::with_capacity(total_len);
    pkt.extend_from_slice(&DIRECTION_HOST_TO_DEVICE);
    pkt.extend_from_slice(&command);
    pkt.extend_from_slice(&(total_len as u16).to_le_bytes());
    pkt.extend_from_slice(body);
    let checksum = calculate_checksum(&pkt);
    pkt.push(checksum);
    pkt
}

pub fn build_state_request_packet() -> Vec<u8> {
    encode_host_packet(CMD_STATE_REQUEST, &[])
}

pub fn build_sound_modes_packet(modes: &A3959SoundModes) -> Vec<u8> {
    let manual_clamped = modes.manual_anc_level.clamp(1, 5);
    let adaptive_clamped = modes.adaptive_anc_level.clamp(0, 5);
    let combined_level = (adaptive_clamped << 4) | (manual_clamped & 0x0F);
    let body = [
        modes.ambient_mode.to_id(),
        combined_level,
        modes.ambient_mode.to_id(),
        modes.anc_mode.to_id(),
        u8::from(modes.wind_noise_suppression),
        modes.adaptive_sensitivity,
        modes.multi_scene_anc.to_id(),
    ];
    encode_host_packet(CMD_SET_SOUND_MODES, &body)
}

pub fn build_surround_sound_packet(enabled: bool) -> Vec<u8> {
    encode_host_packet(CMD_SET_SURROUND_SOUND, &[u8::from(enabled)])
}

pub fn build_gaming_mode_packet(enabled: bool) -> Vec<u8> {
    encode_host_packet(CMD_SET_GAMING_MODE, &[u8::from(enabled)])
}

pub fn build_touch_tone_packet(enabled: bool) -> Vec<u8> {
    encode_host_packet(CMD_SET_TOUCH_TONE, &[u8::from(enabled)])
}

pub fn build_button_action_packet(
    side: EarbudSide,
    press: ButtonPressKind,
    action: ButtonAction,
) -> Vec<u8> {
    let action_byte = match action.to_action_nibble() {
        Some(nibble) => 0xF0 | (nibble & 0x0F),
        None => 0xFF,
    };
    let body = [side.to_id(), press.to_id(), action_byte];
    encode_host_packet(CMD_SET_BUTTON_ACTION, &body)
}

/// Computes the exact 8-band DRC (Dynamic Range Compression) compensation bytes
/// discovered in OpenSCQ30 (`set_equalizer_with_drc.rs`) for Anker Soundcore A3959.
/// Input: `gains_tenths_db` in range `-120..=120` (where `-60` = `-6.0 dB`).
/// Output: 8 wire bytes centered at `120` (`0.0 dB`).
pub fn compute_a3959_drc_bytes(gains_tenths_db: &[i16; 8]) -> [u8; 8] {
    let a: [f64; 8] = std::array::from_fn(|i| gains_tenths_db[i].clamp(-120, 120) as f64);
    const S: f64 = 0.85;
    const L: f64 = 0.95;

    let matrix: [[f64; 8]; 8] = [
        [1.26, -0.71 * S, 0.177, -0.0494, 0.0345, -0.0197, 0.0075, -0.00217],
        [-0.71 * S, 1.73 * L, -0.81 * S, 0.204, -0.068, 0.045, -0.0235, 0.0075],
        [0.177, -0.81 * S, 1.73 * L, -0.81 * S, 0.208, -0.07, 0.045, -0.0197],
        [-0.0494, 0.204, -0.81 * S, 1.73 * L, -0.82 * S, 0.208, -0.068, 0.0345],
        [0.0345, -0.068, 0.208, -0.82 * S, 1.73 * L, -0.81 * S, 0.204, -0.0494],
        [-0.0197, 0.045, -0.07, 0.208, -0.81 * S, 1.73 * L, -0.81 * S, 0.177],
        [0.0075, -0.0235, 0.045, -0.068, 0.204, -0.81 * S, 1.83 * L, -0.71 * S],
        [-0.00217, 0.0075, -0.0197, 0.0345, -0.0494, 0.177, -0.71 * S, 1.5],
    ];

    std::array::from_fn(|row| {
        let sum: f64 = (0..8).map(|col| matrix[row][col] * a[col]).sum();
        let drc_tenths = (sum / 10.0).round() as i32;
        (120 + drc_tenths).clamp(0, 240) as u8
    })
}

/// Builds the 28-byte `02 83` Set Equalizer with DRC packet for Soundcore R50i NC (A3959).
pub fn build_equalizer_with_drc_packet(preset_id: u16, gains_tenths_db: &[i16; 8]) -> Vec<u8> {
    let raw_8: [u8; 8] = std::array::from_fn(|i| {
        let clamped = gains_tenths_db[i].clamp(-120, 120);
        (120 + clamped) as u8
    });
    let drc_8 = compute_a3959_drc_bytes(gains_tenths_db);

    let mut body = Vec::with_capacity(18);
    body.extend_from_slice(&preset_id.to_le_bytes());
    body.extend_from_slice(&raw_8);
    body.extend_from_slice(&drc_8);
    encode_host_packet(CMD_SET_EQUALIZER_DRC, &body)
}

pub fn parse_a3959_state_packet(packet: &[u8]) -> Result<A3959State, String> {
    if packet.len() < 10 {
        return Err(format!("Packet too short: {} bytes", packet.len()));
    }
    if packet[0..5] != DIRECTION_DEVICE_TO_HOST {
        return Err("Invalid packet direction header".to_string());
    }
    let declared_len = u16::from_le_bytes([packet[7], packet[8]]) as usize;
    if packet.len() < declared_len || declared_len < 10 {
        return Err(format!(
            "Declared length {} does not match buffer length {}",
            declared_len,
            packet.len()
        ));
    }
    let expected_checksum = calculate_checksum(&packet[..declared_len - 1]);
    let actual_checksum = packet[declared_len - 1];
    if expected_checksum != actual_checksum {
        return Err(format!(
            "Checksum mismatch: expected 0x{:02X}, got 0x{:02X}",
            expected_checksum, actual_checksum
        ));
    }

    let body = &packet[9..declared_len - 1];
    if body.len() < 79 {
        return Err(format!("State body too short: {} bytes", body.len()));
    }

    let host_device = body[0];
    let tws_connected = body[1] != 0;
    let left_battery_level = body[2].min(10);
    let right_battery_level = body[3].min(10);
    let left_charging = body[4] == 1;
    let right_charging = body[5] == 1;

    let firmware_left = String::from_utf8_lossy(&body[6..11]).trim().to_string();
    let firmware_right = String::from_utf8_lossy(&body[11..16]).trim().to_string();
    let serial_number = String::from_utf8_lossy(&body[16..32]).trim().to_string();

    let eq_preset_id = u16::from_le_bytes([body[32], body[33]]);
    let mut eq_gains_tenths_db = [0i16; 8];
    for i in 0..8 {
        eq_gains_tenths_db[i] = (body[34 + i] as i16) - 120;
    }
    let drc_bytes = compute_a3959_drc_bytes(&eq_gains_tenths_db);
    let drc_preview_tenths_db = std::array::from_fn(|i| (drc_bytes[i] as i16) - 120);

    let buttons = A3959ButtonConfig {
        left_single: ButtonAction::from_raw_byte(body[56], tws_connected, true),
        right_single: ButtonAction::from_raw_byte(body[57], tws_connected, true),
        left_double: ButtonAction::from_raw_byte(body[58], tws_connected, false),
        right_double: ButtonAction::from_raw_byte(body[59], tws_connected, false),
        left_triple: ButtonAction::from_raw_byte(body[60], tws_connected, true),
        right_triple: ButtonAction::from_raw_byte(body[61], tws_connected, true),
        left_long: ButtonAction::from_raw_byte(body[62], tws_connected, false),
        right_long: ButtonAction::from_raw_byte(body[63], tws_connected, false),
    };

    let ambient_cycle_mask = body[64];
    let ambient_mode = AmbientSoundMode::from_id(body[65]);
    let combined_anc_level = body[66];
    let adaptive_anc_level = (combined_anc_level >> 4) & 0x0F;
    let manual_anc_level = (combined_anc_level & 0x0F).clamp(1, 5);
    let anc_mode = AncMode::from_id(body[68]);
    let wind_noise_suppression = body[69] != 0;
    let adaptive_sensitivity = body[70];
    let multi_scene_anc = MultiSceneAnc::from_id(body[71]);

    let touch_tone = body[73] != 0;
    let dual_connection = body[74] != 0;
    let onboard_surround = body[75] != 0;
    let auto_power_off_enabled = body[76] != 0;
    let auto_power_off_index = body[77];
    let low_battery_prompt = body[78] != 0;

    let fw_supports_gaming =
        parse_fw_number(&firmware_left) >= 160 || parse_fw_number(&firmware_right) >= 160;
    let gaming_mode = if fw_supports_gaming && body.len() > 79 {
        Some(body[79] != 0)
    } else {
        None
    };

    Ok(A3959State {
        connected: true,
        simulated: false,
        device_name: "Soundcore R50i NC (A3959)".to_string(),
        mac_address: "Connected via RFCOMM".to_string(),
        host_device,
        tws_connected,
        left_battery_level,
        right_battery_level,
        left_battery_percent: left_battery_level * 10,
        right_battery_percent: right_battery_level * 10,
        left_charging,
        right_charging,
        firmware_left,
        firmware_right,
        serial_number,
        eq_preset_id,
        eq_gains_tenths_db,
        drc_preview_tenths_db,
        bass_up_enabled: eq_preset_id == BASS_UP_PRESET_ID,
        buttons,
        ambient_cycle_mask,
        sound_modes: A3959SoundModes {
            ambient_mode,
            manual_anc_level,
            adaptive_anc_level,
            anc_mode,
            wind_noise_suppression,
            adaptive_sensitivity,
            multi_scene_anc,
        },
        touch_tone,
        dual_connection,
        onboard_surround,
        auto_power_off_enabled,
        auto_power_off_index,
        low_battery_prompt,
        gaming_mode,
    })
}

fn parse_fw_number(fw: &str) -> u32 {
    let digits: String = fw.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openscq30_set_equalizer_with_drc_vector() {
        const EXPECTED: &[u8] = &[
            0x08, 0xee, 0x00, 0x00, 0x00, 0x02, 0x83, 0x1c, 0x00, 0xfe, 0xfe, 0x3c, 0xb4, 0x8f,
            0xf0, 0x8e, 0x00, 0x74, 0x88, 0x6d, 0x86, 0x70, 0x88, 0x7b, 0x66, 0x7e, 0x79, 0x4f,
        ];
        let gains = [-60, 60, 23, 120, 22, -120, -4, 16];
        let packet = build_equalizer_with_drc_packet(0xFEFE, &gains);
        assert_eq!(packet.as_slice(), EXPECTED);
    }

    #[test]
    fn test_parse_a3959_state_update_packet_issue_149() {
        const RAW_STATE_PACKET: &[u8] = &[
            0x09, 0xff, 0x00, 0x00, 0x01, 0x01, 0x01, 0x65, 0x00, 0x01, 0x00, 0x06, 0x09, 0xff,
            0xff, 0x30, 0x31, 0x2e, 0x36, 0x34, 0x30, 0x31, 0x2e, 0x36, 0x34, 0x33, 0x39, 0x35,
            0x39, 0x36, 0x38, 0x35, 0x39, 0x32, 0x33, 0x42, 0x46, 0x30, 0x36, 0x37, 0x43, 0xfe,
            0xfe, 0x6a, 0x74, 0x75, 0x77, 0x79, 0x78, 0x77, 0x74, 0x78, 0x78, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8a, 0xf1, 0xf0, 0x66, 0x63, 0x15,
            0x19, 0x64, 0x64, 0x07, 0x00, 0x55, 0x00, 0x01, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x14,
        ];

        let state = parse_a3959_state_packet(RAW_STATE_PACKET).expect("valid state packet");
        assert_eq!(state.host_device, 1);
        assert!(!state.tws_connected);
        assert_eq!(state.left_battery_level, 6);
        assert_eq!(state.right_battery_level, 9);
        assert_eq!(state.left_battery_percent, 60);
        assert_eq!(state.right_battery_percent, 90);
        assert!(!state.left_charging);
        assert!(!state.right_charging);
        assert_eq!(state.firmware_left, "01.64");
        assert_eq!(state.firmware_right, "01.64");
        assert_eq!(state.serial_number, "3959685923BF067C");
        assert_eq!(state.eq_preset_id, 0xFEFE);
        assert_eq!(state.eq_gains_tenths_db, [-14, -4, -3, -1, 1, 0, -1, -4]);
        // Because tws_connected is false (0x00), single and triple presses are disabled:
        assert_eq!(state.buttons.left_single, ButtonAction::Disabled);
        assert_eq!(state.buttons.right_single, ButtonAction::Disabled);
        assert_eq!(state.buttons.left_double, ButtonAction::PlayPause);
        assert_eq!(state.buttons.right_double, ButtonAction::NextSong);
        assert_eq!(state.buttons.left_triple, ButtonAction::Disabled);
        assert_eq!(state.buttons.right_triple, ButtonAction::Disabled);
        assert_eq!(state.buttons.left_long, ButtonAction::AmbientSoundMode);
        assert_eq!(state.buttons.right_long, ButtonAction::AmbientSoundMode);
        assert_eq!(
            state.sound_modes.ambient_mode,
            AmbientSoundMode::NoiseCanceling
        );
        assert_eq!(state.sound_modes.manual_anc_level, 5);
        assert_eq!(state.sound_modes.adaptive_anc_level, 5);
        assert_eq!(state.sound_modes.anc_mode, AncMode::Adaptive);
        assert!(!state.sound_modes.wind_noise_suppression);
        assert_eq!(state.sound_modes.adaptive_sensitivity, 5);
        assert_eq!(state.sound_modes.multi_scene_anc, MultiSceneAnc::Transport);
        assert!(!state.touch_tone);
        assert!(state.dual_connection);
        assert!(!state.onboard_surround);
        assert!(state.auto_power_off_enabled);
        assert_eq!(state.auto_power_off_index, 0);
        assert!(state.low_battery_prompt);
        assert_eq!(state.gaming_mode, Some(false));
    }

    #[test]
    fn test_rejects_corrupted_checksum_and_old_firmware_gaming_mode() {
        let mut raw: Vec<u8> = vec![
            0x09, 0xff, 0x00, 0x00, 0x01, 0x01, 0x01, 0x65, 0x00, 0x01, 0x01, 0x06, 0x09, 0x01,
            0x00, 0x30, 0x31, 0x2e, 0x35, 0x30, 0x30, 0x31, 0x2e, 0x35, 0x30, 0x33, 0x39, 0x35,
            0x39, 0x36, 0x38, 0x35, 0x39, 0x32, 0x33, 0x42, 0x46, 0x30, 0x36, 0x37, 0x43, 0xfe,
            0xfe, 0x6a, 0x74, 0x75, 0x77, 0x79, 0x78, 0x77, 0x74, 0x78, 0x78, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0xf1, 0xf0, 0x66, 0x63, 0x15,
            0x19, 0x64, 0x64, 0x07, 0x00, 0x55, 0x00, 0x01, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,
        ];
        assert!(parse_a3959_state_packet(&raw).is_err());

        let len = raw.len();
        raw[len - 1] = calculate_checksum(&raw[..len - 1]);
        let parsed = parse_a3959_state_packet(&raw).expect("valid checksum");
        assert_eq!(parsed.firmware_left, "01.50");
        assert_eq!(parsed.gaming_mode, None);
        assert!(parsed.left_charging);
        assert!(!parsed.right_charging);
        assert_eq!(parsed.buttons.left_single, ButtonAction::VolumeDown);
        assert_eq!(parsed.buttons.right_single, ButtonAction::VolumeUp);
    }

    #[test]
    fn test_outbound_sound_modes_and_button_packets() {
        let modes = A3959SoundModes {
            ambient_mode: AmbientSoundMode::NoiseCanceling,
            manual_anc_level: 2,
            adaptive_anc_level: 2,
            anc_mode: AncMode::Manual,
            wind_noise_suppression: true,
            adaptive_sensitivity: 255,
            multi_scene_anc: MultiSceneAnc::Outdoor,
        };
        let pkt = build_sound_modes_packet(&modes);
        assert_eq!(
            pkt.as_slice(),
            &[
                0x08, 0xEE, 0x00, 0x00, 0x00, 0x06, 0x81, 0x11, 0x00, 0x00, 0x22, 0x00, 0x00,
                0x01, 0xFF, 0x01, 0xB1
            ]
        );

        let btn_pkt = build_button_action_packet(
            EarbudSide::Right,
            ButtonPressKind::Double,
            ButtonAction::NextSong,
        );
        assert_eq!(
            btn_pkt.as_slice(),
            &[0x08, 0xEE, 0x00, 0x00, 0x00, 0x04, 0x81, 0x0D, 0x00, 0x01, 0x00, 0xF3, 0x7C]
        );
    }
}
