//! On the board's machine, for one call helper on another computer: `herdr-projects voice listen --from <computer>`, run
//! by the helper over SSH. It says when a call rings (`{"ring": id}`); the helper answers how surely you are at that
//! computer (`{"bid": id, "score": n}`: herdr's window in front, input a moment ago); a moment later the computer with
//! the best answer is told to open the call (`{"open": link}`), and when that call ends (`{"ended": id}`). Nothing but
//! a call's link ever leaves this way, and only to a computer that can already run commands here over SSH. A call
//! rung while this computer's own herdr window for that board is in use (here.rs) is this computer's: its own helper
//! listens here too (`from` this computer's name) and takes it at once, and no other computer is told of it.
use super::ring::ringing;
use crate::clock::now;
use crate::json::*;
use crate::voice::call;
use std::collections::HashMap;

/// How long the helpers have to answer a ring before the best one is told to open it; and how long the best one has
/// to claim it before another may (one that claimed it and could not open it leaves the board's link to you).
pub const BID_MS: i64 = 900;
pub const LATE_MS: i64 = 4000;
fn bids_dir(id: &str) -> std::path::PathBuf { call::dir().join("bids").join(id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect::<String>()) }
fn clean(s: &str) -> String { s.chars().filter(|c| c.is_ascii_alphanumeric() || "_.-".contains(*c)).take(64).collect() }
pub fn bid(id: &str, from: &str, score: i64) { let _ = crate::store::files::write_json(&bids_dir(id).join(format!("{}.json", clean(from))), &json!({ "from": clean(from), "score": score, "at": now() })); }
/// The computer with the best answer: the highest score, then the first name.
pub fn best(id: &str) -> Option<String> {
    let mut all: Vec<(i64, String)> = std::fs::read_dir(bids_dir(id)).ok()?.flatten().filter_map(|e| crate::store::files::read_obj(&e.path())).map(|b| (b.i("score"), b.t("from"))).collect();
    all.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    all.into_iter().next().map(|x| x.1)
}
/// One listener: for which computer (`from`), on which one (`here`, this one's name), whether the human is at this
/// computer's own herdr window for a session (`local`), what it saw ring, when, and what it opened.
pub struct Listener { pub from: String, here: String, local: Box<dyn Fn(&str) -> bool>, seen: HashMap<String, (i64, bool)>, opened: Option<String>, alive_at: i64 }
impl Listener {
    pub fn new(from: &str) -> Self { Self::with(from, &crate::remote::check::hostname(), Box::new(super::here::local_active)) }
    pub fn with(from: &str, here: &str, local: Box<dyn Fn(&str) -> bool>) -> Self { Listener { from: clean(from), here: clean(here), local, seen: HashMap::new(), opened: None, alive_at: 0 } }
    /// One look: a line from the helper (or none), and what to say back.
    pub fn step(&mut self, line: Option<&str>) -> Vec<Value> {
        let mut out = vec![];
        let t = now();
        if let Some(v) = line.and_then(|l| serde_json::from_str::<Value>(l).ok()) { if v.b("bid") { bid(v.s("bid"), &self.from, v.i("score")); } }
        if let Some((id, link, offered, session)) = ringing() {
            // Asked once a call: is the human at this computer's own herdr window for the board that rang?
            let first = !self.seen.contains_key(&id);
            if first { let own = (self.local)(&session); self.seen.insert(id.clone(), (t, own)); }
            let (at, own) = self.seen[&id];
            let is_here = self.from == self.here;
            // This computer's call: no other computer hears of it; this computer's own helper takes it at once.
            if own && !is_here { return self.tail(out, t); }
            if first { if !own { bid(&id, &self.from, 0); } out.push(json!({ "ring": id })); }
            let mine = best(&id).as_deref() == Some(self.from.as_str());
            if !offered && (own || (t - at >= BID_MS && (mine || t - at >= LATE_MS))) && call::change(|c| if c.has("offered") { false } else { c.put("offered", self.from.clone()); true }).unwrap_or(false) {
                self.opened = Some(id.clone());
                out.push(json!({ "open": link, "id": id }));
            }
        }
        self.tail(out, t)
    }
    /// The end of a look: a call this listener opened that ended, and now and then a sign of life.
    fn tail(&mut self, mut out: Vec<Value>, t: i64) -> Vec<Value> {
        if let Some(id) = self.opened.clone() { if call::get().map(|c| c.t("id") != id).unwrap_or(true) { self.opened = None; out.push(json!({ "ended": id })); } }
        if t - self.alive_at > 15000 { self.alive_at = t; out.push(json!({ "alive": t, "version": crate::store::files::VERSION })); }
        out
    }
}
/// The command: listen until the helper goes (its end of the connection closes).
pub fn listen_main(from: &str) -> i32 {
    use std::io::{BufRead, Write};
    let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
    std::thread::spawn(move || { for l in std::io::stdin().lock().lines() { if tx.send(l.ok()).is_err() { return; } } let _ = tx.send(None); });
    let mut l = Listener::new(from);
    let mut stdout = std::io::stdout();
    loop {
        let line = match rx.recv_timeout(std::time::Duration::from_millis(300)) { Ok(Some(x)) => Some(x), Ok(None) => return 0, Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None, Err(_) => return 0 };
        for v in l.step(line.as_deref()) { if writeln!(stdout, "{v}").and_then(|_| stdout.flush()).is_err() { return 0; } }
    }
}
