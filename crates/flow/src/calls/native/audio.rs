//! The call's sound on this computer: your microphone as 16 kHz 16-bit mono chunks, and the voice agent's sound played
//! as it comes, stopped at once when you talk over it (`flush`). On Linux the sound server's own tools do it (PipeWire's
//! `pw-record`/`pw-play`, or PulseAudio's `parec`/`pacat`, which PipeWire also answers), through the sound server's
//! WebRTC echo cancelling where it can load it for the call (`module-echo-cancel`, unloaded after): mono at 16 kHz on
//! the very microphone and speaker in use (a mono microphone fed to a stereo canceller came out at a third of its
//! level, measured on PipeWire), with its gain control on. Without it you need headphones, and the chat says so. On
//! macOS: mac.rs.
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;

/// 20 ms of 16 kHz 16-bit mono.
pub const CHUNK: usize = 640;
pub trait Audio: Send {
    /// Your microphone, chunk by chunk.
    fn mic(&mut self) -> Vec<Vec<u8>>;
    fn play(&mut self, pcm: &[u8]);
    /// Stop what plays now (you talked over it).
    fn flush(&mut self);
    /// Whether the sound is echo cancelled: without it, headphones.
    fn aec(&self) -> bool;
    /// What runs the sound, for `herdr-projects voice`.
    fn how(&self) -> String;
    fn stop(&mut self);
}
fn has(prog: &str) -> bool { Command::new("sh").args(["-c", &format!("command -v {prog}")]).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) }
fn quiet(prog: &str, args: &[&str]) -> Option<String> { let o = Command::new(prog).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?; o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string()) }
/// The sound of this computer, or why there is none.
pub fn open() -> Result<Box<dyn Audio>, String> {
    if cfg!(target_os = "macos") { return super::mac::open(); }
    Linux::open().map(|l| Box::new(l) as Box<dyn Audio>)
}
pub struct Linux { rec: Child, mic: mpsc::Receiver<Vec<u8>>, player: Option<Child>, play_cmd: (String, Vec<String>), module: Option<String>, tools: String }
const SOURCE: &str = "herdr_call_mic";
const SINK: &str = "herdr_call_out";
/// The microphone and the speaker in use now.
fn defaults() -> (String, String) {
    let get = |what: &str, info: &str| quiet("pactl", &[&format!("get-default-{what}")]).filter(|x| !x.is_empty() && !x.starts_with('@'))
        .or_else(|| quiet("pactl", &["info"]).and_then(|i| i.lines().find_map(|l| l.strip_prefix(info).map(|x| x.trim().to_string())))).unwrap_or_default();
    (get("source", "Default Source:"), get("sink", "Default Sink:"))
}
/// The sound server's echo cancelling for this call: mono at 16 kHz on the microphone and speaker in use, its gain
/// control on (PipeWire's words for it, else PulseAudio's, else none). Its module's id, or None where it cannot load.
pub fn load_echo_cancel() -> Option<String> {
    let (source, sink) = defaults();
    let mut base: Vec<String> = vec!["load-module".into(), "module-echo-cancel".into(), "aec_method=webrtc".into(), format!("source_name={SOURCE}"), format!("sink_name={SINK}"), "channels=1".into(), "rate=16000".into()];
    if !source.is_empty() { base.push(format!("source_master={source}")); }
    if !sink.is_empty() { base.push(format!("sink_master={sink}")); }
    for gain in ["aec_args=webrtc.gain_control=true", "aec_args=analog_gain_control=0 digital_gain_control=1", ""] {
        let mut a = base.clone();
        if !gain.is_empty() { a.push(gain.into()); }
        let args: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
        if let Some(id) = quiet("pactl", &args).filter(|m| !m.is_empty() && m.chars().all(|c| c.is_ascii_digit())) { return Some(id); }
    }
    None
}
impl Linux {
    pub fn open() -> Result<Linux, String> {
        let pulse = has("parec") && has("pacat");
        let pw = has("pw-record") && has("pw-play");
        if !pulse && !pw { return Err("No sound tools here: a call needs PipeWire's pw-record and pw-play (or PulseAudio's parec and pacat).".into()); }
        // The sound server's own echo cancelling, for this call: a microphone and a speaker of its own (one left from a
        // call that did not end cleanly is taken away first).
        if has("pactl") { for l in quiet("pactl", &["list", "short", "modules"]).unwrap_or_default().lines().filter(|l| l.contains(SOURCE)) { if let Some(id) = l.split_whitespace().next() { let _ = quiet("pactl", &["unload-module", id]); } } }
        let module = if has("pactl") { load_echo_cancel() } else { None };
        let (src, sink) = if module.is_some() { (Some(SOURCE), Some(SINK)) } else { (None, None) };
        let fmt = |dev: Option<&str>, flag: &str, mut base: Vec<String>| { if let Some(d) = dev { base.push(format!("{flag}={d}")); } base };
        let (rec_cmd, play_cmd) = if pulse {
            (("parec".to_string(), fmt(src, "--device", vec!["--format=s16le".into(), "--rate=16000".into(), "--channels=1".into(), "--latency-msec=20".into()])),
             ("pacat".to_string(), fmt(sink, "--device", vec!["--playback".into(), "--format=s16le".into(), "--rate=16000".into(), "--channels=1".into(), "--latency-msec=60".into()])))
        } else {
            let mut r = fmt(src, "--target", vec!["--raw".into(), "--format=s16".into(), "--rate=16000".into(), "--channels=1".into()]); r.push("-".into());
            let mut p = fmt(sink, "--target", vec!["--raw".into(), "--format=s16".into(), "--rate=16000".into(), "--channels=1".into()]); p.push("-".into());
            (("pw-record".to_string(), r), ("pw-play".to_string(), p))
        };
        let mut rec = Command::new(&rec_cmd.0).args(&rec_cmd.1).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| format!("{} did not start: {e}", rec_cmd.0))?;
        let (tx, mic) = mpsc::channel();
        let mut out = rec.stdout.take().unwrap();
        std::thread::spawn(move || { let mut buf = [0u8; CHUNK]; loop { if out.read_exact(&mut buf).is_err() { return; } if tx.send(buf.to_vec()).is_err() { return; } } });
        let tools = format!("{} and {}{}", rec_cmd.0, play_cmd.0, if module.is_some() { ", with the sound server's echo cancelling" } else { ", no echo cancelling" });
        let mut l = Linux { rec, mic, player: None, play_cmd, module, tools };
        l.player = l.spawn_player();
        Ok(l)
    }
    fn spawn_player(&self) -> Option<Child> { Command::new(&self.play_cmd.0).args(&self.play_cmd.1).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok() }
}
impl Audio for Linux {
    fn mic(&mut self) -> Vec<Vec<u8>> { self.mic.try_iter().collect() }
    fn play(&mut self, pcm: &[u8]) {
        if self.player.as_ref().map(|p| p.stdin.is_none()).unwrap_or(true) { self.player = self.spawn_player(); }
        if let Some(i) = self.player.as_mut().and_then(|p| p.stdin.as_mut()) { if i.write_all(pcm).is_err() { self.player = None; } }
    }
    fn flush(&mut self) { if let Some(mut p) = self.player.take() { let _ = p.kill(); let _ = p.wait(); } self.player = self.spawn_player(); }
    fn aec(&self) -> bool { self.module.is_some() }
    fn how(&self) -> String { self.tools.clone() }
    fn stop(&mut self) {
        if let Some(mut p) = self.player.take() { let _ = p.kill(); let _ = p.wait(); }
        let _ = self.rec.kill(); let _ = self.rec.wait();
        if let Some(m) = self.module.take() { let _ = quiet("pactl", &["unload-module", &m]); }
    }
}
impl Drop for Linux { fn drop(&mut self) { self.stop(); } }
