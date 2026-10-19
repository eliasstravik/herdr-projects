//! The call's sound on macOS: a small program of Apple's own audio (mac_audio.swift, built into this program and
//! compiled once with the Xcode Command Line Tools' `swiftc`, kept in the data folder by its hash), with Apple's voice
//! processing, which cancels the echo of the speakers. It writes the microphone to its standard output and plays what
//! it reads (audio.rs: Audio). macOS asks once whether the terminal herdr runs in may use the microphone.
use super::audio::{Audio, CHUNK};
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

const SOURCE: &str = include_str!("mac_audio.swift");
pub struct Mac { child: Child, mic: mpsc::Receiver<Vec<u8>>, noaec: Arc<AtomicBool> }
/// The compiled program, made once for this source.
fn program() -> Result<std::path::PathBuf, String> {
    let dir = crate::store::files::data_dir().join("voice").join("bin");
    let hash = sha1_smol::Sha1::from(SOURCE).digest().to_string();
    let bin = dir.join(format!("herdr-call-audio-{}", &hash[..10]));
    if bin.exists() { return Ok(bin); }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src = dir.join(format!("herdr-call-audio-{}.swift", &hash[..10]));
    std::fs::write(&src, SOURCE).map_err(|e| e.to_string())?;
    let out = Command::new("swiftc").args(["-O", "-o"]).arg(&bin).arg(&src).stdin(Stdio::null()).output().map_err(|_| "A call on macOS needs the Xcode Command Line Tools (for swiftc): run xcode-select --install, then call again.".to_string())?;
    if !out.status.success() { return Err(format!("The call's sound program did not build: {}", crate::text::cut(String::from_utf8_lossy(&out.stderr).trim(), 300))); }
    Ok(bin)
}
pub fn open() -> Result<Box<dyn Audio>, String> {
    let bin = program()?;
    let mut child = Command::new(bin).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("The call's sound program did not start: {e}"))?;
    let (tx, mic) = mpsc::channel();
    let mut out = child.stdout.take().unwrap();
    std::thread::spawn(move || { let mut buf = [0u8; CHUNK]; loop { if out.read_exact(&mut buf).is_err() { return; } if tx.send(buf.to_vec()).is_err() { return; } } });
    // A line "noaec" on its standard error: the voice processing could not be had.
    let noaec = Arc::new(AtomicBool::new(false));
    let (flag, err) = (noaec.clone(), child.stderr.take().unwrap());
    std::thread::spawn(move || { use std::io::BufRead; for l in std::io::BufReader::new(err).lines().map_while(Result::ok) { if l.contains("noaec") { flag.store(true, Ordering::SeqCst); } } });
    Ok(Box::new(Mac { child, mic, noaec }))
}
impl Mac { fn frame(&mut self, kind: u8, body: &[u8]) { if let Some(i) = self.child.stdin.as_mut() { let mut f = vec![kind]; f.extend_from_slice(&(body.len() as u32).to_le_bytes()); f.extend_from_slice(body); let _ = i.write_all(&f); } } }
impl Audio for Mac {
    fn mic(&mut self) -> Vec<Vec<u8>> { self.mic.try_iter().collect() }
    fn play(&mut self, pcm: &[u8]) { self.frame(b'a', pcm); }
    fn flush(&mut self) { self.frame(b'f', &[]); }
    fn aec(&self) -> bool { !self.noaec.load(Ordering::SeqCst) }
    fn how(&self) -> String { format!("macOS voice processing{}", if self.aec() { ", with echo cancelling" } else { ", no echo cancelling" }) }
    fn stop(&mut self) { drop(self.child.stdin.take()); let _ = self.child.kill(); let _ = self.child.wait(); }
}
impl Drop for Mac { fn drop(&mut self) { self.stop(); } }
