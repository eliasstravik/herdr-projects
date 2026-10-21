//! The call helper, on the computer you sit at: `herdr-projects voice helper`. It listens on every machine herdr has
//! saved (the ones the board reaches over SSH: remote.rs), each over one SSH connection of its own (listen.rs runs at
//! the other end), and on this computer itself, for a board that runs here; it says how surely you are here when a call
//! rings (here.rs), and when this computer gets the call it holds it right here, with this computer's microphone and
//! speaker and no browser (native/). One per computer; the keeper starts it while herdr has saved machines and
//! `voice.helper` is `auto`, and starts it again when it stopped. With `voice.calls off` it never runs.
use super::listen::{Door, Listener};
use super::native::{audio, board::Pipe, engine::Engine, ws};
use crate::json::*;
use crate::remote::{machines, ssh_command, Machine};
use crate::store::files::data_dir;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

fn beat_file() -> std::path::PathBuf { data_dir().join("voice").join("helper.json") }
/// Alive means it wrote its heartbeat in the last half minute.
pub fn alive() -> bool { std::fs::metadata(beat_file()).and_then(|m| m.modified()).map(|m| m.elapsed().map(|e| e.as_secs() < 30).unwrap_or(true)).unwrap_or(false) }
pub fn wanted() -> bool { crate::voice::calls_on() && crate::store::settings::lookup::setting_text("voice.helper", None) != "none" && !machines().is_empty() }
/// Started by the keeper, when it should run and does not.
pub fn ensure() { if wanted() && !alive() && crate::store::marks::due("voice-helper-start", 20000) { crate::housekeeping::spawn_loose(&["voice", "helper"]); } }
fn this_computer() -> String { crate::remote::check::hostname() }
/// A call this computer got: its sound here, its talk with ElevenLabs, its board over `pipe`. A call that cannot be
/// held here (no sound, no connection) is ended with why, which the board then says.
fn hold(call: Value, pipe: Arc<Pipe>, hangup: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let started = audio::open().and_then(|a| ws::connect(call.s("url")).map(|w| (a, w)));
        let (a, w) = match started { Ok(x) => x, Err(why) => { use super::native::board::Board; pipe.ask("end", json!({ "why": format!("The call could not be held here: {why}") })); return; } };
        let mut e = Engine::new(Box::new(w), a, pipe, call.s("target"));
        e.hangup = hangup;
        e.run();
    });
}
/// What a listener said, handled the same for another machine's board and this computer's own: a bid, a call to hold,
/// an answer for the call, its end. `bid`: how the answer to a ring goes back.
fn handle(v: &Value, machine: &str, pipe: &Arc<Pipe>, hangup: &mut Arc<AtomicBool>, bid: &dyn Fn(String)) {
    if v.b("ring") { bid(json!({ "bid": v.s("ring"), "score": super::here::score(machine) }).to_string()); }
    if v.has("call") { *hangup = Arc::new(AtomicBool::new(false)); hold(v.at("call").clone(), pipe.clone(), hangup.clone()); }
    if v.b("ended") { hangup.store(true, Ordering::SeqCst); }
    pipe.answer(v);
}
/// One machine, listened to for as long as the helper runs: again after a pause when the connection ends.
fn listen_to(m: Machine) {
    let mut wait = 5;
    loop {
        let (cmd, args) = ssh_command(&m, &format!("herdr-projects voice listen --from {}", crate::remote::clean(&this_computer(), "_.-")), false);
        let started = std::time::Instant::now();
        if let Ok(mut ch) = Command::new(cmd).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() {
            let input = Arc::new(Mutex::new(ch.stdin.take()));
            let w = input.clone();
            let pipe = Pipe::new(Box::new(move |line| w.lock().unwrap().as_mut().map(|i| writeln!(i, "{line}").and_then(|_| i.flush()).is_ok()).unwrap_or(false)));
            let mut hangup = Arc::new(AtomicBool::new(false));
            let send = |line: String| { if let Some(i) = input.lock().unwrap().as_mut() { let _ = writeln!(i, "{line}"); let _ = i.flush(); } };
            for line in BufReader::new(ch.stdout.take().unwrap()).lines().map_while(Result::ok) {
                handle(&serde_json::from_str(&line).unwrap_or(Value::Null), &m.label, &pipe, &mut hangup, &send);
            }
            hangup.store(true, Ordering::SeqCst);
            let _ = ch.kill(); let _ = ch.wait();
        }
        // A connection that held for a while was fine: try again soon. One that keeps failing: less and less often.
        wait = if started.elapsed().as_secs() > 60 { 5 } else { (wait * 2).min(120) };
        std::thread::sleep(std::time::Duration::from_secs(wait));
    }
}
/// This computer's own board: listened to in this process, as `voice listen` listens for another computer.
fn listen_here(door: Door) {
    let me = this_computer();
    let mut l = Listener::new(&me, door);
    let (tx, rx) = mpsc::channel::<String>();
    let t = Mutex::new(tx);
    let pipe = Pipe::new(Box::new(move |line| t.lock().unwrap().send(line).is_ok()));
    let mut hangup = Arc::new(AtomicBool::new(false));
    let mut pending: Vec<String> = vec![];
    loop {
        pending.extend(rx.try_iter());
        let line = if pending.is_empty() { None } else { Some(pending.remove(0)) };
        let bids = std::cell::RefCell::new(vec![]);
        for v in l.step(line.as_deref()) { handle(&v, &me, &pipe, &mut hangup, &|b| bids.borrow_mut().push(b)); }
        pending.extend(bids.into_inner());
        if pending.is_empty() { std::thread::sleep(std::time::Duration::from_millis(50)); }
    }
}
pub fn helper_main(door: Door) -> i32 {
    if !crate::voice::calls_on() { return 0; }
    let Ok(_held) = crate::store::files::hold_lock("voice-helper", 0) else { return 0 }; // one per computer
    let mut listening: Vec<String> = vec![];
    let (exe, stamp) = { let e = crate::store::files::bin(); let s = crate::keeper::code_stamp(&e); (e, s) };
    std::thread::spawn(move || listen_here(door));
    loop {
        // Updated, or no longer wanted: it goes (the keeper starts the new one).
        if !wanted() || crate::keeper::code_stamp(&exe) != stamp { let _ = std::fs::remove_file(beat_file()); return 0; }
        for m in machines() { if !listening.contains(&m.target) { listening.push(m.target.clone()); std::thread::spawn(move || listen_to(m)); } }
        let _ = crate::store::files::write_json(&beat_file(), &json!({ "pid": std::process::id(), "machines": listening, "at": crate::clock::now() }));
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
