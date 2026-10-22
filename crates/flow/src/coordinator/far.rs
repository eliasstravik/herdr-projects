//! Another machine's Coordinator: what a machine sends of its own Coordinator with its tasks (where it stands and the
//! words of its conversation, kept small), and that read back on the asking side as a look the board's chat draws as it
//! draws this machine's. A machine on an older version sends none, and a machine with no Coordinator says so.
use super::look::Look;
use crate::agents::talk::Said;
use crate::json::*;

/// How many things said, and how long each may be, travel with the list.
const KEEP: usize = 12;
const LONGEST: usize = 6000;
fn cut(s: &str) -> String { if s.chars().count() <= LONGEST { s.to_string() } else { format!("{} …", s.chars().take(LONGEST).collect::<String>()) } }
/// This machine's Coordinator as another machine is sent it. `ready`: one can run here at all.
pub fn sent(v: &Look, ready: bool) -> Value {
    if !ready { return json!({ "off": true }); }
    let talk: Vec<Value> = v.talk.iter().skip(v.talk.len().saturating_sub(KEEP)).map(|m| json!({ "you": m.you, "text": cut(&m.text) })).collect();
    // Its pane there: a file attached for it goes into its folder through the web UI (webui/attach/far.rs).
    json!({ "state": v.state, "error": v.error, "queue": v.queue, "readable": v.readable, "said": cut(&v.said), "sent": cut(&v.sent), "talk": talk, "pane": v.pane,
        "began": v.began, "at": v.at })
}
/// What another machine sent of its Coordinator, as a look. None: it sent nothing (an older version), or has none.
pub fn read(v: &Value) -> Option<Look> {
    if !v.is_object() || v.b("off") { return None; }
    let talk = v.arr("talk").iter().map(|m| Said { you: m.b("you"), text: m.t("text"), at: 0 }).filter(|m| !m.text.is_empty()).collect();
    // `status` only says whether its program answered: a readable conversation is read whatever it is.
    // What its row says came with it (an older version sends none: its row then has no time).
    let l = Look { state: v.t("state"), error: v.t("error"), queue: v.at("queue").as_u64().unwrap_or(0) as usize, readable: v.b("readable"), status: Some(v.t("state")), said: v.t("said"), sent: v.t("sent"), talk, pane: v.t("pane"), began: v.i("began"), at: v.i("at"), ..Default::default() };
    Some(Look { asks: l.asking(), ..l })
}
