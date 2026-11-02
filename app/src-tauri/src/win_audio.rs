//! Windows: which output is playing right now, and playing the announcement.

use crate::device::Output;
use crate::store::Kind;

/// Classifies an endpoint from its audio device instance path (like `BTHENUM\...`, `USB\VID_...`,
/// `HDAUDIO\FUNC_01...`) and its form factor (1 speakers, 3 headphones, 5 headset).
pub fn classify_endpoint(endpoint_id: &str, name: &str, instance: &str, form_factor: u32) -> Output {
    let up = instance.to_uppercase();
    let (kind, address) = if up.contains("BTHENUM") || up.contains("BTHHFENUM") || up.contains("BTHLE") {
        (Kind::Bluetooth, bt_address_from_instance(&up))
    } else if up.contains("USB") {
        (Kind::Usb, None)
    } else if form_factor == 3 || form_factor == 5 {
        (Kind::Wired, None)
    } else {
        (Kind::Builtin, None)
    };
    // Bluetooth and USB names carry the product in brackets; the jack and speakers keep their full name
    let name = if matches!(kind, Kind::Bluetooth | Kind::Usb) { clean_name(name) } else { name.trim().to_string() };
    Output { kind, key: endpoint_id.to_string(), name, address }
}

/// `...&0&A0E9DB123456_C00000000` holds the MAC as the 12 hex digits before `_`.
fn bt_address_from_instance(up: &str) -> Option<String> {
    let tail = up.rsplit('&').next()?;
    let hex = tail.split('_').next()?;
    (hex.len() == 12 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| hex.as_bytes().chunks(2).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(":"))
}

/// "Headphones (Soundcore R50i NC)" reads better as "Soundcore R50i NC".
fn clean_name(name: &str) -> String {
    match (name.find('('), name.rfind(')')) {
        (Some(a), Some(b)) if b > a + 1 && b == name.len() - 1 => name[a + 1..b].trim().to_string(),
        _ => name.trim().to_string(),
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::{GUID, HSTRING, PCWSTR};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Foundation::PROPERTYKEY;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, PKEY_AudioEndpoint_FormFactor, PlaySoundW, SND_ASYNC,
        SND_FILENAME, SND_NODEFAULT,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};

    /// The audio devnode's instance path, as exposed on the endpoint's property store.
    const PKEY_DEVICE_INSTANCE: PROPERTYKEY =
        PROPERTYKEY { fmtid: GUID::from_u128(0xb3f8fa53_0004_438e_9003_51a46e139bfc), pid: 2 };

    pub fn default_output() -> Option<Output> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()?;
            let id = dev.GetId().ok()?.to_string().ok()?;
            let props = dev.OpenPropertyStore(STGM_READ).ok()?;
            let name = props.GetValue(&PKEY_Device_FriendlyName).map(|v| v.to_string()).unwrap_or_default();
            let instance = props.GetValue(&PKEY_DEVICE_INSTANCE).map(|v| v.to_string()).unwrap_or_default();
            let ff = props.GetValue(&PKEY_AudioEndpoint_FormFactor).ok().and_then(|v| u32::try_from(&v).ok()).unwrap_or(0);
            Some(classify_endpoint(&id, &name, &instance, ff))
        }
    }

    /// Calls `on_change` with the default output whenever it changes.
    // ponytail: polls once a second; switch to IMMNotificationClient if 1 s ever feels slow
    pub fn watch(on_change: impl Fn(Option<Output>) + Send + 'static) {
        std::thread::spawn(move || {
            let mut last: Option<Option<Output>> = None;
            loop {
                let now = default_output();
                if last.as_ref() != Some(&now) {
                    on_change(now.clone());
                    last = Some(now);
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
    }

    pub fn play_wav(path: &std::path::Path) {
        let p = HSTRING::from(path.as_os_str());
        unsafe {
            let _ = PlaySoundW(PCWSTR(p.as_ptr()), None, SND_FILENAME | SND_ASYNC | SND_NODEFAULT);
        }
    }
}

#[cfg(windows)]
pub use imp::{default_output, play_wav, watch};

#[cfg(test)]
mod tests {
    use super::*;

    const EP: &str = "{0.0.0.00000000}.{11111111-2222-3333-4444-555555555555}";

    #[test]
    fn bluetooth_instance_maps_to_bluetooth_with_address() {
        let o = classify_endpoint(EP, "Headphones (Soundcore R50i NC)", r"{1}.BTHENUM\{0000110B-0000-1000-8000-00805F9B34FB}_VID&0001009E_PID&4020\8&2C9A1C3&0&A0E9DB123456_C00000000", 3);
        assert_eq!(o.kind, Kind::Bluetooth);
        assert_eq!(o.address.as_deref(), Some("A0:E9:DB:12:34:56"));
        assert_eq!(o.name, "Soundcore R50i NC");
    }

    #[test]
    fn usb_instance_maps_to_usb() {
        assert_eq!(classify_endpoint(EP, "Speakers (USB Audio)", r"{1}.USB\VID_0D8C&PID_0014&MI_00\7&1", 1).kind, Kind::Usb);
    }

    #[test]
    fn headphone_form_factor_maps_to_wired() {
        assert_eq!(classify_endpoint(EP, "Headphones (Realtek(R) Audio)", r"{1}.HDAUDIO\FUNC_01&VEN_10EC", 3).kind, Kind::Wired);
    }

    #[test]
    fn speaker_form_factor_maps_to_builtin() {
        let o = classify_endpoint(EP, "Speakers (Realtek(R) Audio)", r"{1}.HDAUDIO\FUNC_01&VEN_10EC", 1);
        assert_eq!(o.kind, Kind::Builtin);
        assert_eq!(o.name, "Speakers (Realtek(R) Audio)");
        assert_eq!(o.key, EP);
    }
}
