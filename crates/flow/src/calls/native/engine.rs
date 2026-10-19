//! One call, held here: your microphone goes to ElevenLabs as it comes, the voice agent's sound is played as it comes
//! and stopped the moment you talk over it (its interruption), the voice agent's tools are run by the board, and what
//! either side said is told to the board in order, each tool only after what was said before it. What to read back is
//! asked of the board every second and a half and said only in a pause: never while you talk, never over the voice
//! agent. Words of yours that never reached the board stop every message until you said the whole request again. The
//! call ends when either side hangs up, when the voice agent said goodbye, or when the connection goes.
use super::audio::Audio;
use super::board::Board;
use super::wire::{self, Event, Gate};
use super::ws::Wire;
use crate::json::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

/// A pause: nobody spoke for this long. The feed is asked this often, and how the sound stands told this often.
pub const QUIET_MS: i64 = 1400;
const FEED_MS: i64 = 1500;
const REPORT_MS: i64 = 7500;
pub struct Engine {
    wire: Box<dyn Wire>, audio: Box<dyn Audio>, board: Arc<dyn Board>, gate: Gate,
    said: mpsc::Sender<(String, String, u64)>, told: Arc<AtomicU64>, said_n: u64, lost: Arc<Mutex<&'static str>>,
    results: (mpsc::Sender<(String, String)>, mpsc::Receiver<(String, String)>), fed: (mpsc::Sender<Value>, mpsc::Receiver<Value>),
    queue: Vec<String>, feeding: Arc<AtomicBool>, heard_at: i64, play_until: i64, fed_at: i64, report_at: i64, end_at: Option<i64>, peak: u16,
    pub hangup: Arc<AtomicBool>, pub ended: Option<String>,
}
fn now() -> i64 { crate::clock::now() }
impl Engine {
    pub fn new(wire: Box<dyn Wire>, audio: Box<dyn Audio>, board: Arc<dyn Board>, target: &str) -> Engine {
        let (said, todo) = mpsc::channel::<(String, String, u64)>();
        let (told, lost) = (Arc::new(AtomicU64::new(0)), Arc::new(Mutex::new("")));
        // What was said goes to the board one after the other, tried three times; a loss of your words is kept.
        let (b, t, l) = (board.clone(), told.clone(), lost.clone());
        std::thread::spawn(move || for (who, text, n) in todo {
            let ok = (0..3).any(|_| b.ask("heard", json!({ "who": who, "text": text })).0 == 200);
            let mut lost = l.lock().unwrap();
            if who == "user" { if !ok { *lost = "lost"; } else if *lost == "asked" { *lost = ""; } }
            t.store(n, Ordering::SeqCst);
        });
        let mut e = Engine { wire, audio, board, gate: Gate::default(), said, told, said_n: 0, lost, results: mpsc::channel(), fed: mpsc::channel(), queue: vec![], feeding: Arc::new(AtomicBool::new(false)),
            heard_at: 0, play_until: 0, fed_at: now(), report_at: now() - REPORT_MS, end_at: None, peak: 0, hangup: Arc::new(AtomicBool::new(false)), ended: None };
        if let Err(err) = e.wire.send(wire::hello(target)) { e.ended = Some(err); }
        e
    }
    fn say(&mut self, who: &str, text: &str) { self.said_n += 1; let _ = self.said.send((who.into(), text.into(), self.said_n)); }
    /// A tool of the voice agent's: run by the board after what was said before it reached the board.
    fn tool(&mut self, call: String, name: String, args: Value) {
        let (board, told, upto, lost, tx) = (self.board.clone(), self.told.clone(), self.said_n, self.lost.clone(), self.results.0.clone());
        if name == "end_call" { self.end_at = Some(now() + 2500); }
        std::thread::spawn(move || {
            for _ in 0..1000 { if told.load(Ordering::SeqCst) >= upto { break; } std::thread::sleep(std::time::Duration::from_millis(10)); }
            let refused = name == "send_message" && { let mut l = lost.lock().unwrap(); if l.is_empty() { false } else { *l = "asked"; true } };
            let result = if refused { "Not sent: some of the human's words did not reach the board. Ask them to say their whole request again, then send it.".to_string() } else {
                let (st, v) = board.ask("tool", json!({ "name": name, "args": args }));
                if st == 200 { v.t("result") } else { v.t("msg") }
            };
            let _ = tx.send((call, result));
        });
    }
    fn event(&mut self, e: Event, t: i64) -> Result<(), String> {
        match e {
            Event::Audio { id, pcm } => if self.gate.allows(id) && !pcm.is_empty() {
                for c in pcm.chunks(2) { if c.len() == 2 { self.peak = self.peak.max(i16::from_le_bytes([c[0], c[1]]).unsigned_abs()); } }
                self.audio.play(&pcm);
                self.play_until = self.play_until.max(t) + (pcm.len() as i64) / 32;
            },
            Event::Interruption { id } => { self.gate.interrupt(id); self.audio.flush(); self.play_until = t; }
            Event::Ping { id } => self.wire.send(wire::pong(id))?,
            Event::Tool { call, name, args } => self.tool(call, name, args),
            Event::Agent(text) => if !text.is_empty() && !text.starts_with("[board]") { self.say("agent", &text); },
            Event::User(text) => { self.heard_at = t; if !text.is_empty() { self.say("user", &text); } }
            Event::Vad(score) => if score > 0.5 { self.heard_at = t; },
            Event::Started { .. } | Event::Other(_) => {}
        }
        Ok(())
    }
    /// One turn of the loop at time `t`: what came in, your audio out, answers, the feed, and the end.
    pub fn step(&mut self, t: i64) -> bool {
        if self.ended.is_some() { return false; }
        let r = (|| -> Result<bool, String> {
            let mut busy = false;
            for _ in 0..20 { match self.wire.recv()? { Some(text) => { busy = true; self.event(wire::parse(&text), t)?; } None => break } }
            for chunk in self.audio.mic() { busy = true; self.wire.send(wire::audio(&chunk))?; }
            while let Ok((call, result)) = self.results.1.try_recv() { self.wire.send(wire::tool_result(&call, &result))?; }
            while let Ok(f) = self.fed.1.try_recv() {
                if f.b("ended") { return Err("ended".into()); }
                self.queue.extend(f.arr("items").iter().map(|i| i.t("text")).filter(|x| !x.is_empty()));
            }
            if t - self.fed_at >= FEED_MS && !self.feeding.swap(true, Ordering::SeqCst) {
                self.fed_at = t;
                let (b, tx, flag) = (self.board.clone(), self.fed.0.clone(), self.feeding.clone());
                std::thread::spawn(move || { let (st, v) = b.ask("feed", json!({})); if st == 410 { let _ = tx.send(json!({ "ended": true })); } else if st == 200 { let _ = tx.send(v); } flag.store(false, Ordering::SeqCst); });
            }
            if !self.queue.is_empty() && t > self.play_until + QUIET_MS && t - self.heard_at > QUIET_MS {
                let item = self.queue.remove(0);
                self.wire.send(wire::board_says(&item))?;
                self.play_until = t;
            }
            if t - self.report_at >= REPORT_MS { self.report_at = t; let (b, r) = (self.board.clone(), self.report()); std::thread::spawn(move || { b.ask("audio", r); }); }
            if self.hangup.load(Ordering::SeqCst) { return Err("hung up".into()); }
            if self.end_at.map(|e| t >= e).unwrap_or(false) { self.board.ask("end", json!({ "why": "said goodbye", "audio": self.report() })); return Err("said goodbye".into()); }
            Ok(busy)
        })();
        match r {
            Ok(busy) => busy,
            Err(why) => {
                if why != "ended" && why != "said goodbye" && why != "hung up" { self.board.ask("end", json!({ "why": "the connection went", "audio": self.report() })); }
                self.wire.close(); self.audio.stop(); self.ended = Some(why); false
            }
        }
    }
    /// The updates waiting for a pause.
    pub fn queued(&self) -> usize { self.queue.len() }
    /// How the sound stood, for the board (`herdr-projects voice`).
    pub fn report(&self) -> Value { json!({ "via": "native", "aec": self.audio.aec(), "how": self.audio.how(), "peak": (self.peak as f64 / 32768.0 * 1000.0).round() / 1000.0, "elements": 1, "playing": 1, "agent": crate::remote::check::hostname() }) }
    /// Run until the call ends; answers why.
    pub fn run(mut self) -> String {
        while self.ended.is_none() { if !self.step(now()) { std::thread::sleep(std::time::Duration::from_millis(5)); } }
        self.ended.unwrap_or_default()
    }
}
