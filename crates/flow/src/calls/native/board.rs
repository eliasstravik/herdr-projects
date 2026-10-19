//! The board, from the helper's side of a call: its requests (`heard`, `tool`, `feed`, `audio`, `end`) go as numbered
//! lines to the listener on the board's machine (over the helper's SSH connection, or in this process for a board of
//! this computer: listen.rs), which answers each with its number; the call's key never leaves the board's machine.
use crate::json::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

pub trait Board: Send + Sync { fn ask(&self, what: &str, body: Value) -> (u16, Value); }
type Write = Box<dyn Fn(String) -> bool + Send + Sync>;
pub struct Pipe { write: Write, waiting: Mutex<HashMap<i64, mpsc::Sender<(u16, Value)>>>, n: AtomicI64 }
impl Pipe {
    /// `write`: sends one line to the listener (false: it is gone).
    pub fn new(write: Write) -> Arc<Pipe> { Arc::new(Pipe { write, waiting: Mutex::new(HashMap::new()), n: AtomicI64::new(0) }) }
    /// A line from the listener: an answer to one of ours is handed to whoever waits for it.
    pub fn answer(&self, v: &Value) -> bool {
        if !v.has("res") { return false; }
        if let Some(tx) = self.waiting.lock().unwrap().remove(&v.i("res")) { let _ = tx.send((v.i("status") as u16, v.at("body").clone())); }
        true
    }
}
impl Board for Pipe {
    fn ask(&self, what: &str, body: Value) -> (u16, Value) {
        let n = self.n.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = mpsc::channel();
        self.waiting.lock().unwrap().insert(n, tx);
        if !(self.write)(json!({ "req": n, "what": what, "body": body }).to_string()) { self.waiting.lock().unwrap().remove(&n); return (502, json!({ "ok": false, "msg": "The board's machine is not connected." })); }
        rx.recv_timeout(std::time::Duration::from_secs(60)).unwrap_or((504, json!({ "ok": false, "msg": "The board did not answer." })))
    }
}
