#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

pub const APP_NAME: &str = "SoundBettrCore";
pub const APP_EXE: &str = "SoundBettrCore.exe";
pub const VERSION: &str = "0.1.0";
pub const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\SoundBettrCore";

static EMBEDDED_APP_EXE: &[u8] = include_bytes!(
    "C:/Users/J30N/LodiSave/app/target/release/soundbettrcore.exe"
);

const DEFAULT_HARMAN_APO_TXT: &str = "\
# SoundBettrCore EqualizerAPO Live Chain
# Profile: R50i NC Harman Reference (r50i_harman_reference)
Preamp: -2.40 dB
Filter 1: ON LSC Fc 31.0 Hz Gain 1.50 dB Q 0.71
Filter 2: ON PK Fc 62.0 Hz Gain -2.00 dB Q 1.41
Filter 3: ON PK Fc 125.0 Hz Gain -4.50 dB Q 1.41
Filter 4: ON PK Fc 250.0 Hz Gain -2.50 dB Q 1.41
Filter 5: ON PK Fc 500.0 Hz Gain 0.00 dB Q 1.41
Filter 6: ON PK Fc 1000.0 Hz Gain 1.00 dB Q 1.41
Filter 7: ON PK Fc 2000.0 Hz Gain 3.00 dB Q 1.41
Filter 8: ON PK Fc 4000.0 Hz Gain 3.50 dB Q 1.41
Filter 9: ON PK Fc 8000.0 Hz Gain 1.50 dB Q 1.41
Filter 10: ON HSC Fc 16000.0 Hz Gain 2.50 dB Q 0.71
";

fn default_install_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Programs")
        .join(APP_NAME)
}

#[cfg(windows)]
fn init_com() {
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

#[cfg(windows)]
fn create_shortcut(lnk: &Path, target: &Path, description: &str) -> Result<(), String> {
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface};

    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| e.to_string())?;
        link.SetPath(&HSTRING::from(target.as_os_str()))
            .map_err(|e| e.to_string())?;
        if let Some(dir) = target.parent() {
            let _ = link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()));
        }
        let _ = link.SetDescription(&HSTRING::from(description));
        let _ = link.SetIconLocation(&HSTRING::from(target.as_os_str()), 0);
        let file: IPersistFile = link.cast().map_err(|e| e.to_string())?;
        file.Save(&HSTRING::from(lnk.as_os_str()), true)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn write_uninstall_registry(install_dir: &Path, uninstaller_path: &Path) {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, REG_SZ, RegCloseKey, RegCreateKeyW, RegSetValueExW,
    };
    use windows::core::HSTRING;

    unsafe {
        let mut key = HKEY::default();
        if RegCreateKeyW(
            HKEY_CURRENT_USER,
            &HSTRING::from(UNINSTALL_KEY),
            &mut key,
        )
        .is_ok()
        {
            let set_sz = |name: &str, val: &str| {
                let bytes: Vec<u8> = val
                    .encode_utf16()
                    .chain(Some(0))
                    .flat_map(|c| c.to_le_bytes())
                    .collect();
                let _ = RegSetValueExW(key, &HSTRING::from(name), None, REG_SZ, Some(&bytes));
            };
            set_sz("DisplayName", "SoundBettrCore (Soundcore R50i NC Studio)");
            set_sz("DisplayVersion", VERSION);
            set_sz("Publisher", "Jhems");
            set_sz("InstallLocation", &install_dir.display().to_string());
            set_sz(
                "DisplayIcon",
                &install_dir.join(APP_EXE).display().to_string(),
            );
            set_sz(
                "UninstallString",
                &format!("\"{}\" --uninstall", uninstaller_path.display()),
            );
            let _ = RegCloseKey(key);
        }
    }
}

fn configure_equalizer_apo() {
    if let Ok(program_data) = std::env::var("ProgramData") {
        let dir = Path::new(&program_data).join("SoundBettrCore");
        if fs::create_dir_all(&dir).is_ok() {
            let _ = fs::write(dir.join("live_apo.txt"), DEFAULT_HARMAN_APO_TXT);
        }
    }
    let eapo_dir = Path::new(r"C:\Program Files\EqualizerAPO\config");
    if eapo_dir.exists() {
        let inc = eapo_dir.join("soundbettrcore.txt");
        if fs::write(&inc, DEFAULT_HARMAN_APO_TXT).is_ok() {
            let main_cfg = eapo_dir.join("config.txt");
            if let Ok(existing) = fs::read_to_string(&main_cfg) {
                if !existing.contains("soundbettrcore.txt") {
                    let next = format!("{}\nInclude: soundbettrcore.txt\n", existing.trim_end());
                    let _ = fs::write(&main_cfg, next);
                }
            }
        }
    }
}

fn perform_install(install_dir: &Path, silent: bool) -> Result<PathBuf, String> {
    fs::create_dir_all(install_dir).map_err(|e| e.to_string())?;
    let target_exe = install_dir.join(APP_EXE);
    if target_exe.exists() {
        let _ = Command::new(&target_exe).arg("--quit").status();
    }
    fs::write(&target_exe, EMBEDDED_APP_EXE).map_err(|e| e.to_string())?;

    let uninstaller_exe = install_dir.join("Uninstall SoundBettrCore.exe");
    if let Ok(self_exe) = std::env::current_exe() {
        let _ = fs::copy(&self_exe, &uninstaller_exe);
    }

    configure_equalizer_apo();

    #[cfg(windows)]
    {
        init_com();
        if let Ok(appdata) = std::env::var("APPDATA") {
            let start_menu = Path::new(&appdata)
                .join(r"Microsoft\Windows\Start Menu\Programs\SoundBettrCore.lnk");
            let _ = create_shortcut(
                &start_menu,
                &target_exe,
                "SoundBettrCore Spatial & Harman Audio Rack for Soundcore R50i NC",
            );
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let desktop = Path::new(&userprofile).join(r"Desktop\SoundBettrCore.lnk");
            let _ = create_shortcut(
                &desktop,
                &target_exe,
                "SoundBettrCore Spatial & Harman Audio Rack for Soundcore R50i NC",
            );
        }
        write_uninstall_registry(install_dir, &uninstaller_exe);

        if !silent {
            use windows::Win32::UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK, MessageBoxW};
            use windows::core::{HSTRING, w};
            let msg = format!(
                "SoundBettrCore {} has been installed to:\n{}\n\nEqualizerAPO Harman Reference profile initialized.\nLaunching SoundBettrCore Studio Rack now.",
                VERSION,
                install_dir.display()
            );
            unsafe {
                let _ = MessageBoxW(
                    None,
                    &HSTRING::from(msg),
                    w!("SoundBettrCore Setup"),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            let _ = Command::new(&target_exe).spawn();
        }
    }

    Ok(target_exe)
}

fn perform_uninstall(install_dir: &Path) -> ExitCode {
    let target_exe = install_dir.join(APP_EXE);
    if target_exe.exists() {
        let _ = Command::new(&target_exe).arg("--quit").status();
    }
    let _ = fs::remove_file(&target_exe);
    if let Ok(appdata) = std::env::var("APPDATA") {
        let _ = fs::remove_file(
            Path::new(&appdata).join(r"Microsoft\Windows\Start Menu\Programs\SoundBettrCore.lnk"),
        );
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let _ = fs::remove_file(Path::new(&userprofile).join(r"Desktop\SoundBettrCore.lnk"));
    }
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteTreeW};
        use windows::core::HSTRING;
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(UNINSTALL_KEY));
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--verify-payload") {
        if EMBEDDED_APP_EXE.len() > 100_000 && &EMBEDDED_APP_EXE[0..2] == b"MZ" {
            println!(
                "SoundBettrCore Setup v{VERSION} payload verified: {} bytes (PE MZ header OK)",
                EMBEDDED_APP_EXE.len()
            );
            return ExitCode::SUCCESS;
        }
        return ExitCode::FAILURE;
    }

    let install_dir = args
        .iter()
        .position(|a| a == "--dir")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(default_install_dir);

    if args.iter().any(|a| a == "--uninstall") {
        return perform_uninstall(&install_dir);
    }

    let silent = args.iter().any(|a| a == "--silent");
    match perform_install(&install_dir, silent) {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
