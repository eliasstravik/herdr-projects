//! `herdr-projects web-ui`: how the board in the web UI (Ovis) stands, and the one setup step by hand.
//!   web-ui           where the web UI is, whether the front door is up, and which links go through it
//!   web-ui link      point the web UI's shared link at the front door (the yes the board asks for)
//!   web-ui unlink    point it back; the board does not ask again (`web-ui link` does it later)
//! `remove-extras` runs `leave`: the link goes back and the front door stops.
use super::find::{web_ui, WebUi, OLD, OVIS};
use super::keep::{door_file, door_info, ensure_door};
use super::link::{self, address_of};
use crate::extras::{extras, note};
use crate::json::*;
use crate::store::files::read_json;

/// What the board asks about once: the web UI's private links that do not go through the front door yet (none: no
/// question).
pub fn offer() -> Vec<String> {
    if extras().b("webUi") { return vec![]; }
    let (Some(to), Some(door)) = (web_ui(true), door_info()) else { return vec![] };
    link::look(to.port, Some(door.i("port") as u16)).direct.into_iter().map(|e| e.url).collect()
}
/// No: the link is left alone, and the board does not ask again.
pub fn decline() { note(json!({ "webUi": "no" })); }
pub fn link_now() -> Value {
    if extras().s("webUi") == "removed" { note(json!({ "webUi": "" })); } // asked for again
    let Some(to) = web_ui(true) else { return json!({ "ok": false, "msg": format!("The web UI (Ovis) is not installed in herdr (its plugin {OVIS}, or {OLD} before the rename), or `board.webUi` is `none`: there is nothing to show the board in.") }) };
    ensure_door();
    let Some(door) = door_info() else { return json!({ "ok": false, "msg": "The front door is not up yet. Try again in a few seconds." }) };
    let urls = link::link(to.port, door.i("port") as u16);
    let msg = if urls.is_empty() { format!("The web UI has no private link yet. Make one with its own Phone setup; it then goes through the front door by itself. On this machine the Board is at {}/.", address_of(door.i("port") as u16)) } else { format!("The Board is in the web UI at {}. `herdr-projects web-ui unlink` points it back.", urls.join(", ")) };
    json!({ "ok": true, "urls": urls, "msg": msg })
}
pub fn unlink_now(answer: &str) -> Value {
    let d = read_json(&door_file()).unwrap_or(Value::Null);
    let to = web_ui(true);
    note(json!({ "webUi": answer })); // first: whatever comes of the link, the front door must not take it again
    let web = to.map(|w| w.port).or_else(|| (d.at("to").i("port") > 0).then(|| d.at("to").i("port") as u16));
    match link::unlink((d.i("port") > 0).then(|| d.i("port") as u16), web) {
        Err(msg) => json!({ "ok": false, "msg": msg }),
        Ok(n) => json!({ "ok": true, "n": n, "msg": if n > 0 { format!("The web UI's link points at the web UI again ({n} link{}).", if n == 1 { "" } else { "s" }) } else { "No link went through the front door.".into() } }),
    }
}
/// Everything taken out: the link goes back, and the page server (the front door with it) stops. It is started again
/// on demand, for evidence pages, and opens no front door while the answer is "removed".
pub fn leave() -> Value {
    let out = unlink_now("removed");
    if let Some(s) = crate::review_page::server_info() { unsafe { libc::kill(s.i("pid") as i32, libc::SIGTERM); } }
    out
}
/// The command's first line: where the web UI is, under which of its two ids and version (none for one named by hand),
/// or why it was not found.
pub fn found_line(to: Option<&WebUi>) -> String {
    match to {
        Some(w) => format!("Web UI (Ovis): found at http://{}:{}{}", w.host, w.port, if w.plugin.is_empty() { String::new() } else { format!(" ({})", [w.plugin.as_str(), &w.version].join(" ").trim()) }),
        None => format!("Web UI (Ovis): not found{}", if crate::store::settings::lookup::setting("board.webUi", None) == json!("none") { " (board.webUi is none)".into() } else if extras().s("webUi") == "removed" { " (taken out with remove-extras; `web-ui link` brings it back)".into() } else { format!(": its plugin ({OVIS}, or {OLD} before the rename) is not installed in herdr") }),
    }
}
pub fn main(args: &[String]) -> Value {
    match args.first().map(|s| s.as_str()) {
        Some("link") => return link_now(),
        Some("unlink") => return unlink_now("no"),
        Some(_) => return json!({ "ok": false, "msg": "Usage: herdr-projects web-ui [link|unlink]" }),
        None => {}
    }
    let (to, door) = (web_ui(true), door_info());
    let links = to.as_ref().map(|w| link::look(w.port, door.as_ref().map(|d| d.i("port") as u16)));
    let mut lines = vec![found_line(to.as_ref())];
    lines.push(match &door { Some(d) => format!("Front door: up at {}/ (this machine only)", address_of(d.i("port") as u16)), None => "Front door: not running".into() });
    if let Some(l) = &links {
        for e in &l.ours { lines.push(format!("Link: {} goes through the front door: the Board is in the web UI there", e.url)); }
        for e in &l.direct { lines.push(format!("Link: {} goes straight to the web UI: no Board there (`herdr-projects web-ui link`)", e.url)); }
        for e in &l.open { lines.push(format!("Link: {} is open to the internet (Funnel): the board never goes there", e.url)); }
        if l.ours.is_empty() && l.direct.is_empty() { lines.push("Link: the web UI has no private link yet (its own Settings → Phone makes one)".into()); }
    }
    json!({ "ok": true, "msg": lines.join("\n") })
}
