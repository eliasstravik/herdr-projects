//! A file attached in the web board's chat box for another machine's agent or coordinator, put where that one reads
//! it, on its machine. First the herdr web UI's own way, as its chat box sends a file for a pane on another of its PCs
//! (`POST /api/machines/<PC>/pane/image`, checked on herdr-web-ui 0.4.8): that PC keeps it beside the pane, in
//! `<its folder>/.ovis/` (`.herdr-web-ui/` before its rename to Ovis; a task's worktree leaves both out of git,
//! delegate/worktree.rs). The front door asks with the caller's own headers, so the web UI lets in only whom it lets
//! in itself. Where that cannot be (no PC of the web UI for that machine, not connected, no pane known yet, or the web
//! UI says why not): over the board's own SSH link (remote/carry.rs). The answer is the path there; a file that went
//! neither way says why.
use crate::json::*;
use crate::remote::carry::{send_bytes, For};
use crate::remote::Machine;
use crate::webui::pcs::{pc_of, Pc};
use crate::webui::proxy::{ask_with, caller_headers, Head};

/// The web UI as the front door reaches it: its PCs (asked only for a file that goes to another machine; `pcs_now`
/// not from what was kept, for a pane just reopened: webui/going.rs), and one POST of JSON to a path of its API,
/// answered with its status and what it said (None: it did not answer). Tests stand in for it.
pub trait WebUi { fn pcs(&self) -> Option<Vec<Pc>>; fn pcs_now(&self) -> Option<Vec<Pc>> { self.pcs() } fn post(&self, path: &str, body: &Value) -> Option<(u16, Value)>; }
/// The front door's: the web UI at `to`, asked with the headers of the request the file came in.
pub struct Door<'a> { pub head: &'a Head, pub to: (String, u16) }
/// How long a file may take to reach the other machine through the web UI.
const WAIT_MS: u64 = 120_000;
impl WebUi for Door<'_> {
    fn pcs(&self) -> Option<Vec<Pc>> { crate::webui::pcs::ask_pcs(self.head, &self.to) }
    fn pcs_now(&self) -> Option<Vec<Pc>> { crate::webui::pcs::ask_pcs_now(self.head, &self.to) }
    fn post(&self, path: &str, body: &Value) -> Option<(u16, Value)> {
        let mut h: Vec<(String, String)> = caller_headers(self.head).into_iter().filter(|(k, _)| !k.eq_ignore_ascii_case("x-herdr-machine")).collect();
        h.push(("Content-Type".into(), "application/json".into()));
        // The web UI's own mark of a request its page made on purpose (its machine-security.ts).
        h.push(("X-Herdr-Machine".into(), "1".into()));
        let a = ask_with(&self.to.0, self.to.1, "POST", path, &h, &serde_json::to_vec(body).ok()?, 1 << 20, WAIT_MS)?;
        Some((a.status, serde_json::from_slice(&a.body).unwrap_or(Value::Null)))
    }
}
/// Who on another machine a file is for: that machine, the pane its reader runs in there ("" none known), and who it is
/// as the board's own link names them.
pub struct Far { pub machine: Machine, pub pane: String, pub to: For }
/// Through the web UI: Ok(the path there); Err(None) when it cannot be asked for this one, Err(Some(why)) when it said
/// why not.
fn by_web_ui(f: &Far, ui: Option<&dyn WebUi>, name: &str, kind: &str, bytes: &[u8]) -> Result<String, Option<String>> {
    let Some(ui) = ui.filter(|_| !f.pane.is_empty()) else { return Err(None) };
    let Some(pc) = ui.pcs().and_then(|pcs| pc_of(&f.machine, &pcs)).filter(|pc| pc.connected) else { return Err(None) };
    let path = format!("/api/machines/{}/pane/image", crate::util::url_encode(&pc.id));
    let body = json!({ "pane_id": f.pane, "content_type": kind, "data_base64": crate::util::base64(bytes), "name": name });
    let Some((status, said)) = ui.post(&path, &body) else { return Err(Some("the web UI did not answer".into())) };
    if (200..300).contains(&status) && said.b("path") { return Ok(said.t("path")); }
    let why = said.at("error").t("message");
    Err(Some(if why.is_empty() { format!("the web UI answered {status}") } else { format!("the web UI said: {why}") }))
}
/// The file sent to its machine, the web UI's way or the board's own: its path there.
pub fn send(f: &Far, ui: Option<&dyn WebUi>, name: &str, kind: &str, bytes: &[u8]) -> Result<String, String> {
    let web = match by_web_ui(f, ui, name, kind, bytes) { Ok(path) => return Ok(path), Err(why) => why };
    send_bytes(&f.machine, name, kind, bytes, &f.to).map_err(|e| {
        let web = web.map(|w| format!("{w}; ")).unwrap_or_default();
        format!("The file could not be sent to {}: {web}{}.", f.machine.label, e.trim_end_matches('.'))
    })
}
