//! The call helper, on the computer you sit at: `herdr-projects voice helper`. It listens on every machine herdr has
//! saved (the ones the board reaches over SSH: remote.rs), each over one SSH connection of its own (listen.rs runs at
//! the other end), and on this computer itself, for a board that runs here; it says how surely you are here when a call
//! rings (here.rs), and opens the call's page when this computer is the one (browser.rs), closing its window when the
//! call ends. One per computer; the keeper starts it while herdr has
//! saved machines and `voice.helper` is `auto`, and starts it again when it stopped.
use super::browser;
use crate::json::*;
use crate::remote::{machines, ssh_command, Machine};
use crate::store::files::data_dir;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn beat_file() -> std::path::PathBuf { data_dir().join("voice").join("helper.json") }
/// Alive means it wrote its heartbeat in the last half minute.
pub fn alive() -> bool { std::fs::metadata(beat_file()).and_then(|m| m.modified()).map(|m| m.elapsed().map(|e| e.as_secs() < 30).unwrap_or(true)).unwrap_or(false) }
pub fn wanted() -> bool { crate::store::settings::lookup::setting_text("voice.helper", None) != "none" && !machines().is_empty() }
/// Started by the keeper, when it should run and does not.
pub fn ensure() { if wanted() && !alive() && crate::store::marks::due("voice-helper-start", 20000) { crate::housekeeping::spawn_loose(&["voice", "helper"]); } }
fn this_computer() -> String { crate::remote::check::hostname() }
/// One machine, listened to for as long as the helper runs: again after a pause when the connection ends.
fn listen_to(m: Machine) {
    let mut wait = 5;
    loop {
        let (cmd, args) = ssh_command(&m, &format!("herdr-projects voice listen --from {}", crate::util::shq(&this_computer())), false);
        let started = std::time::Instant::now();
        if let Ok(mut ch) = Command::new(cmd).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() {
            let mut input = ch.stdin.take();
            let mut window = None;
            for line in BufReader::new(ch.stdout.take().unwrap()).lines().map_while(Result::ok) {
                let v: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
                if v.b("ring") { let score = super::here::score(&m.label); if let Some(i) = input.as_mut() { let _ = writeln!(i, "{}", json!({ "bid": v.s("ring"), "score": score })); let _ = i.flush(); } }
                if v.b("open") { browser::close(&mut window); window = browser::open(v.s("open")); }
                if v.b("ended") { browser::close(&mut window); }
            }
            browser::close(&mut window);
            let _ = ch.kill(); let _ = ch.wait();
        }
        // A connection that held for a while was fine: try again soon. One that keeps failing: less and less often.
        wait = if started.elapsed().as_secs() > 60 { 5 } else { (wait * 2).min(120) };
        std::thread::sleep(std::time::Duration::from_secs(wait));
    }
}
/// This computer's own board: listened to in this process, as `voice listen` listens for another computer.
fn listen_here() {
    let me = this_computer();
    let mut l = super::listen::Listener::new(&me);
    let mut window = None;
    let mut bid: Option<String> = None;
    loop {
        for v in l.step(bid.take().as_deref()) {
            if v.b("ring") { bid = Some(json!({ "bid": v.s("ring"), "score": super::here::score(&me) }).to_string()); }
            if v.b("open") { browser::close(&mut window); window = browser::open(v.s("open")); }
            if v.b("ended") { browser::close(&mut window); }
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}
pub fn helper_main() -> i32 {
    let Ok(_held) = crate::store::files::hold_lock("voice-helper", 0) else { return 0 }; // one per computer
    let mut listening: Vec<String> = vec![];
    let (exe, stamp) = { let e = crate::store::files::bin(); let s = crate::keeper::code_stamp(&e); (e, s) };
    std::thread::spawn(listen_here);
    loop {
        // Updated, or no longer wanted: it goes (the keeper starts the new one).
        if !wanted() || crate::keeper::code_stamp(&exe) != stamp { let _ = std::fs::remove_file(beat_file()); return 0; }
        for m in machines() { if !listening.contains(&m.target) { listening.push(m.target.clone()); std::thread::spawn(move || listen_to(m)); } }
        let _ = crate::store::files::write_json(&beat_file(), &json!({ "pid": std::process::id(), "machines": listening, "at": crate::clock::now() }));
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
