//! The call record: one call at a time on this machine, in `data/voice/call.json`, shared by the board that started
//! it, the front door that runs its tools, and the other board, which shows it. Who it talks to (`target`, published by
//! the board that owns the call, as the chat's switch stands), what was said (`lines`, the last ones, shown in the
//! chat), your own words since the last thing sent (`heard`), a switch asked for by voice (`switch`), what was already
//! read back (`fed`), and the secrets that let a page hold this call: a ticket (once, for a short while) and the call's
//! key, both kept only as hashes.
use crate::clock::now;
use crate::json::*;
use crate::store::files::{data_dir, read_obj, with_lock, write_json};
use std::path::PathBuf;

/// A ticket is good for this long, once; a call nobody picked up after that is let go.
pub const TICKET_MS: i64 = 120_000;
/// A live call whose page has not asked the board anything for this long is over: the page went without hanging up
/// (a phone that died, a tab that was killed). A page asks every second or two while its call is on.
pub const STALE_MS: i64 = 60_000;
/// The lines kept for the chat, and your own words kept until something is sent.
const LINES: usize = 40;
const HEARD: usize = 6;

pub fn dir() -> PathBuf { data_dir().join("voice") }
pub fn file() -> PathBuf { dir().join("call.json") }
fn hash(secret: &str) -> String { key_hash(secret) }
/// How a secret of a call is kept: never itself.
pub fn key_hash(secret: &str) -> String { sha1_smol::Sha1::from(format!("herdr-projects-call:{secret}")).digest().to_string() }
/// A secret for a page: 24 random bytes, written as letters a URL keeps whole.
pub fn secret() -> String { crate::util::random_hex(24) }

/// Who the call talks to: a task's agent (`reply`, with its `id`), or the Coordinator of the board in `scope`; `name`
/// as the chat says it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Target { pub id: String, pub reply: bool, pub scope: String, pub name: String }
impl Target {
    pub fn of(v: &Value) -> Target { Target { id: v.t("id"), reply: v.b("reply"), scope: v.t("scope"), name: v.t("name") } }
    pub fn value(&self) -> Value { json!({ "id": self.id, "reply": self.reply, "scope": self.scope, "name": self.name }) }
}

/// The call as it stands, or None when there is none (or it ended, or rang too long with nobody picking up).
pub fn get() -> Option<Obj> {
    let c = read_obj(&file())?;
    if c.s("state") == "ended" || c.s("id").is_empty() { return None; }
    if c.s("state") == "ringing" && now() - c.i("at") > TICKET_MS { return None; }
    if c.s("state") == "live" && now() - c.i("at").max(c.i("answered")).max(c.i("seen")) > STALE_MS { return None; }
    Some(c)
}
/// Change the call under its lock; None when there is none.
pub fn change<T>(f: impl FnOnce(&mut Obj) -> T) -> Option<T> {
    with_lock("voice-call", || {
        let Some(mut c) = get() else { return Ok(None) };
        let out = f(&mut c);
        write_json(&file(), &Value::Object(c))?;
        Ok(Some(out))
    }).ok().flatten()
}
/// A new call, in place of any other. `owner`: "terminal" (the terminal board; a helper on your computer, or a link,
/// picks it up with the ticket) or "web:<page>" (the web board's page that holds it). Answers (the call, its ticket,
/// its key): the two secrets are said once, here, and kept only as hashes.
pub fn start(owner: &str, target: &Target) -> (Obj, String, String) {
    let (ticket, key) = (secret(), secret());
    let id = format!("call-{}", &crate::util::random_hex(4));
    let web = owner.starts_with("web:");
    let c = obj(json!({
        "id": id, "owner": owner, "state": if web { "live" } else { "ringing" }, "at": now(), "target": target.value(),
        "ticket": if web { Value::Null } else { json!(hash(&ticket)) }, "key": hash(&key),
        "lines": [], "heard": [], "fed": {}, "quiet": false,
    }));
    let _ = with_lock("voice-call", || write_json(&file(), &Value::Object(c.clone())));
    (c, ticket, key)
}
/// The ticket a page brought: right, and not used yet. It is used up now, and the call is answered.
pub fn take_ticket(ticket: &str) -> Option<Obj> {
    if ticket.is_empty() { return None; }
    change(|c| {
        if c.s("ticket") != hash(ticket) || now() - c.i("at") > TICKET_MS { return None; }
        c.put("ticket", Value::Null); c.put("link", Value::Null); c.put("state", "live"); c.put("answered", now());
        Some(c.clone())
    }).flatten()
}
/// The page holding the call asked the board something: it is still there.
pub fn seen() { change(|c| c.put("seen", now())); }
/// The call this key holds, when it is the call on now.
pub fn by_key(key: &str) -> Option<Obj> { get().filter(|c| !key.is_empty() && c.s("key") == hash(key)) }
pub fn end(why: &str) {
    let _ = with_lock("voice-call", || {
        let Some(mut c) = read_obj(&file()) else { return Ok(()) };
        if c.s("state") == "ended" { return Ok(()); }
        c.put("state", "ended"); c.put("ended", now()); c.put("why", why);
        write_json(&file(), &Value::Object(c))
    });
}
/// A line of the call, for the chat: `by` "you" (your words), "voice" (the voice agent's), "sent" (what went to the agent
/// you talk to) or "board" (an update read to you). Your words are also kept until something is sent.
pub fn line(by: &str, text: &str) {
    let text: String = crate::text::clean(text).trim().chars().take(2000).collect();
    if text.is_empty() { return; }
    change(|c| {
        let mut lines = c.arr("lines").to_vec();
        lines.push(json!({ "by": by, "text": text, "at": now() }));
        if lines.len() > LINES { lines.drain(..lines.len() - LINES); }
        c.put("lines", lines);
        if by == "you" {
            let mut heard = c.arr("heard").to_vec();
            heard.push(json!(text));
            if heard.len() > HEARD { heard.drain(..heard.len() - HEARD); }
            c.put("heard", heard);
        }
    });
}
/// Your own words since the last thing sent, oldest first; they are cleared once sent.
pub fn heard(c: &Obj) -> Vec<String> { c.arr("heard").iter().map(text).collect() }
pub fn target(c: &Obj) -> Target { Target::of(c.at("target")) }
/// The board that owns the call says who it talks to now.
pub fn set_target(t: &Target) { change(|c| if target(c) != *t { c.put("target", t.value()); }); }
/// A switch asked for by voice, for the board that owns the call to make, as its own switch: `to` as said.
pub fn ask_switch(to: &str) -> i64 { change(|c| { let n = c.at("switch").i("n") + 1; c.put("switch", json!({ "to": to, "n": n, "at": now() })); n }).unwrap_or(0) }
/// Is this call this board's own? The terminal board owns "terminal"; a web page "web:<its id>".
pub fn owned_by(c: &Obj, owner: &str) -> bool { c.s("owner") == owner }
