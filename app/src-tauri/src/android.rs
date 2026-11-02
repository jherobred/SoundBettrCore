//! Android: the Kotlin side (AudioService, AudioEngine) and this Rust core talk over JNI, so connections
//! are handled even when the app's window is closed.

use crate::autoeq::Index;
use crate::core::{self, Core, Platform, Shared};
use crate::device::Output;
use crate::rfcomm::{find_reply, Hardware};
use crate::store::{Device, Kind, Profile, Store};
use crate::voice::{Announcer, KokoroSynth};
use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

static SHARED: OnceLock<Shared> = OnceLock::new();
static VM: OnceLock<JavaVM> = OnceLock::new();
static ENGINE: OnceLock<GlobalRef> = OnceLock::new();
static APP: Mutex<Option<AppHandle>> = Mutex::new(None);
static DATA: OnceLock<PathBuf> = OnceLock::new();

pub fn shared() -> Option<Shared> {
    SHARED.get().cloned()
}

pub fn attach(app: AppHandle) {
    *APP.lock().unwrap() = Some(app);
}

/// Runs `f` with the AudioEngine class on any thread.
fn engine<T>(f: impl FnOnce(&mut JNIEnv, &JClass) -> jni::errors::Result<T>) -> Option<T> {
    let mut env = VM.get()?.attach_current_thread_permanently().ok()?;
    let class = <&JClass>::from(ENGINE.get()?.as_obj());
    let r = f(&mut env, class);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
        return None;
    }
    r.ok()
}

struct AndroidPlatform;

impl Platform for AndroidPlatform {
    fn name(&self) -> &'static str {
        "android"
    }

    fn apply(&self, _: &Store, current: Option<&Device>, profile: Option<&Profile>, bypass: bool) -> Result<(), String> {
        let json = serde_json::json!({
            "enabled": current.is_some() && profile.is_some() && !bypass,
            "gains": profile.map(|p| p.gains).unwrap_or([0.; 10]),
            "preamp": profile.map_or(0., |p| p.preamp_db()),
            "balance": profile.map_or(0., |p| p.balance),
            "surround": profile.is_some_and(|p| p.surround),
        })
        .to_string();
        let err = engine(|env, c| {
            let s = env.new_string(&json)?;
            let r = env.call_static_method(c, "apply", "(Ljava/lang/String;)Ljava/lang/String;", &[JValue::Object(&s)])?.l()?;
            if r.is_null() { Ok(None) } else { Ok(Some(env.get_string(&JString::from(r))?.into())) }
        });
        match err {
            Some(Some(e)) => Err(e),
            Some(None) => Ok(()),
            None => Err("The audio engine is not running".into()),
        }
    }

    fn play(&self, wav: &Path) {
        let path = wav.display().to_string();
        engine(|env, c| {
            let s = env.new_string(&path)?;
            env.call_static_method(c, "play", "(Ljava/lang/String;)V", &[JValue::Object(&s)]).map(|_| ())
        });
    }

    fn hardware(&self, d: &Device) -> Option<Box<dyn Hardware>> {
        (d.kind == Kind::Bluetooth).then(|| Box::new(AndroidHw(d.id.trim_start_matches("bt:").to_string())) as Box<dyn Hardware>)
    }

    fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Some(app) = APP.lock().unwrap().as_ref() {
            let _ = app.emit(event, payload);
        }
    }
}

struct AndroidHw(String);

impl Hardware for AndroidHw {
    fn exchange(&mut self, packet: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let reply = engine(|env, c| {
            let addr = env.new_string(&self.0)?;
            let bytes = env.byte_array_from_slice(packet)?;
            let r = env
                .call_static_method(c, "exchange", "(Ljava/lang/String;[B)[B", &[JValue::Object(&addr), JValue::Object(&bytes)])?
                .l()?;
            if r.is_null() { Ok(None) } else { Ok(Some(env.convert_byte_array(JByteArray::from(r))?)) }
        })
        .ok_or("The earbuds did not accept a control connection.")?;
        Ok(reply.and_then(|buf| find_reply(packet, &buf).map(<[u8]>::to_vec)))
    }
}

fn jstr(env: &mut JNIEnv, s: &JString) -> String {
    env.get_string(s).map(Into::into).unwrap_or_default()
}

/// Called by Kotlin before Tauri starts, and by the background service. Safe to call more than once.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_jhems_soundbettrcore_NativeBridge_init(mut env: JNIEnv, _: JClass, data_dir: JString, autoeq: JString) {
    if SHARED.get().is_some() {
        return;
    }
    if let Ok(vm) = env.get_java_vm() {
        let _ = VM.set(vm);
    }
    if let Ok(c) = env.find_class("com/jhems/soundbettrcore/AudioEngine") {
        if let Ok(g) = env.new_global_ref(JObject::from(c)) {
            let _ = ENGINE.set(g);
        }
    }
    let dir = PathBuf::from(jstr(&mut env, &data_dir));
    let index = Index::load_gz(Path::new(&jstr(&mut env, &autoeq))).unwrap_or_default();
    let _ = DATA.set(dir.clone());
    let core = Core::new(dir.join("soundbettrcore"), index, Err("Preparing the voice".into()), Arc::new(AndroidPlatform));
    let _ = SHARED.set(Arc::new(Mutex::new(core)));
}

/// The Kokoro files are unpacked; load the voice.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_jhems_soundbettrcore_NativeBridge_voiceReady(mut env: JNIEnv, _: JClass, model_dir: JString) {
    let dir = PathBuf::from(jstr(&mut env, &model_dir));
    let (Some(shared), Some(data)) = (SHARED.get(), DATA.get()) else { return };
    let a = KokoroSynth::load(&dir).map(|k| Announcer { synth: Box::new(k), cache_dir: data.join("soundbettrcore/voice-cache") });
    shared.lock().unwrap().set_announcer(a);
}

/// The media route changed. `json` is an Output, or "null" when nothing is playing out.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_jhems_soundbettrcore_NativeBridge_onOutput(mut env: JNIEnv, _: JClass, json: JString) {
    let text = jstr(&mut env, &json);
    let Some(shared) = SHARED.get() else { return };
    let o: Option<Output> = serde_json::from_str(&text).unwrap_or(None);
    core::on_output(shared, o);
}
