//! What a chat shows of a conversation: only its words. What a task's agent said, or its reviewer while the task is
//! under review, and what was sent to it, each under a line that names whose words they are: the agent's or the
//! reviewer's, yours, the Coordinator's or the board's. Never the program's screen (no tool call, no spinner, no
//! input box), but for a program whose conversation cannot be read: then only the words on its screen. A picture in
//! what was sent stands as `[Image 1]`. Read from the task's `snap` (data.rs), this machine's or another's.
use crate::board::keys::key_name;
use crate::board::textbox::images::labelled;
use crate::json::*;
use crate::shown::words::brief;

/// One thing said: whose words (`by`: "" for a note of the board's own), the words, and what kind.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part { pub by: String, pub text: String, pub mine: bool, pub sent: bool, pub raw: bool, pub note: String, pub tone: String, pub whole: String }
#[derive(Clone, Debug, Default)]
pub struct Talk { pub who: String, pub name: String, pub parts: Vec<Part>, pub dialog: bool, pub raw: bool }
/// Who typed what was sent to the program, by the name the chat gives them; as the Coordinator's chat has it, your own
/// words are "You". (A machine that does not say who typed it says "you" for all of it.)
fn sender(who: &str) -> &'static str { match who { "board" => "Board", "coordinator" => "Coordinator", _ => "You" } }
/// What was sent is a brief or a prompt, often long: its beginning stands for it. A picture in it stands as its chip,
/// `[Image 1]`, as it did in the box (textbox/images.rs).
fn sent(text: &str) -> String { brief(&labelled(text), 600, " …") }
/// Your own words stand once: where the conversation already ends with them, that part is yours.
pub fn with_mine(parts: Vec<Part>, mine: &str) -> Vec<Part> {
    if mine.is_empty() { return parts; }
    let said: Vec<Part> = parts.into_iter().filter(|p| p.note.is_empty()).collect();
    let words = mine.trim();
    let head: String = words.chars().take(200).collect();
    // Where the program wrote its own name for a picture it took in (`[Image #39]`), the words are compared with each
    // picture as its chip; else as they are, so other pictures with the same words are another message.
    let chipped: String = labelled(words).chars().take(200).collect();
    let same = said.last().map(|l| { let w = if l.whole.is_empty() { &l.text } else { &l.whole }; l.sent && (w.contains(&head) || w.contains("[Image #") && labelled(w).contains(&chipped)) }).unwrap_or(false);
    // A command (`/clear`) is answered by what it did, which the program writes down at once: it stands with its answer.
    let done = words.starts_with('/') && said.len() >= 2 && said[said.len() - 2].sent && said[said.len() - 2].text == words && !said[said.len() - 1].sent;
    if done { return said; }
    let mut out: Vec<Part> = if same { said[..said.len() - 1].to_vec() } else { said };
    out.push(Part { by: "You".into(), text: labelled(words), whole: words.into(), mine: true, sent: true, ..Default::default() });
    out
}
/// The part a chat's box opens on, and the only one it holds until you scroll up: the newest thing said, whoever said
/// it. Your own words are that only until the answer comes; then the answer alone.
pub fn opens_at(parts: &[Part]) -> usize { parts.len().saturating_sub(1) }
pub fn talk(r: &Obj) -> Talk {
    let tk = r.at("snap").at("talk");
    let who = if tk.b("who") { tk.t("who") } else { crate::flow::talk::talker(r).to_string() };
    let name = if who == "reviewer" { "Reviewer" } else { "Agent" };
    let mut parts = vec![];
    if tk.is_object() && !tk.b("readable") {
        let text = crate::agents::screen_text::screen_text(r.at("snap").s("screen"));
        if !text.is_empty() { parts.push(Part { by: format!("{name}, on its screen now"), text, raw: true, ..Default::default() }); }
    } else {
        let mut list: Vec<Value> = tk.arr("list").to_vec();
        // No conversation to read (its file is gone, or its machine sends none): what the task itself kept.
        if who == "agent" && r.b("said") && !list.iter().any(|m| m.s("who") == "them") { list.push(json!({ "who": "them", "text": r.t("said") })); }
        for m in list {
            if m.s("who") == "them" { parts.push(Part { by: name.into(), text: m.t("text"), ..Default::default() }); }
            else { parts.push(Part { by: sender(m.s("who")).into(), text: sent(m.s("text")), whole: m.t("text"), sent: true, ..Default::default() }); }
        }
    }
    let raw = parts.iter().any(|p| p.raw);
    let dialog = if who == "reviewer" { r.s("stage") == "reviewneeds" } else { r.s("stage") == "working" && if r.b("host") { r.s("state") == "blocked" } else { r.at("live").s("status") == "blocked" } };
    let note = |text: String, kind: &str, tone: &str| Part { by: String::new(), note: kind.into(), tone: tone.into(), text, ..Default::default() };
    if dialog {
        parts.push(note(if who == "reviewer" { format!("The reviewer shows a dialog on its screen that waits for your answer. {} opens the task's space: the reviewer is in its tab \"{}\".", key_name("open"), crate::flow::tabs::reviewer_tab(crate::store::reviews::review_number(r), "")) }
            else { format!("The agent shows a dialog on its screen that waits for your answer: {} opens it.", key_name("open")) }, "dialog", "yellow"));
    } else if !tk.is_object() && r.b("host") && who == "reviewer" {
        parts.push(note(format!("The reviewer is at work. {} runs an older herdr-projects, which does not send what its reviewer says: update it there.", r.s("host")), "old", ""));
    } else if !parts.iter().any(|p| !p.sent) && parts.last().map(|p| p.by != "You").unwrap_or(true) {
        // Your words, with nothing said after them, are the newest thing said: they hold the box until the answer.
        parts.push(note(if who == "reviewer" || r.s("state") == "working" { format!("The {who} has just started: nothing to show yet.") } else { format!("The {who} has said nothing yet.") }, "empty", ""));
    }
    Talk { who, name: name.into(), parts, dialog, raw }
}
/// The newest thing a task's agent (or its reviewer) said, for the task panel.
pub struct Preview { pub who: String, pub text: String, pub raw: bool, pub note: String }
pub fn preview(r: &Obj) -> Preview {
    let k = talk(r);
    let last = k.parts.iter().filter(|p| !p.sent).last().cloned().unwrap_or_default();
    Preview { who: k.who, text: last.text, raw: last.raw, note: last.note }
}
/// The Coordinator's conversation, in the same parts.
pub fn coord_talk(v: &crate::coordinator::look::Look, screen: &str) -> (Vec<Part>, bool, bool, bool) {
    // An autopilot wake is no words of yours: it stands under its own name, as the board's words do, and says only why
    // it woke (autopilot/wake.rs).
    let mut parts: Vec<Part> = v.talk.iter().map(|m| if m.you && crate::autopilot::wake::is_wake(&m.text) { Part { by: "Autopilot".into(), text: sent(&crate::autopilot::wake::said(&m.text)), whole: m.text.clone(), sent: true, ..Default::default() } }
        else if m.you { Part { by: "You".into(), text: sent(&m.text), whole: m.text.clone(), sent: true, ..Default::default() } } else { Part { by: "Coordinator".into(), text: m.text.clone(), ..Default::default() } }).collect();
    if !parts.iter().any(|p| !p.sent) && !v.said.is_empty() { parts.push(Part { by: "Coordinator".into(), text: v.said.clone(), ..Default::default() }); }
    // A program whose conversation cannot be read: the words on its screen.
    if !v.readable && v.status.is_some() { let raw = !screen.is_empty(); return (if raw { vec![Part { by: "Coordinator, on its screen now".into(), text: screen.into(), raw: true, ..Default::default() }] } else { vec![] }, raw, false, false); }
    if v.state == "blocked" { parts.push(Part { note: "dialog".into(), tone: "yellow".into(), text: format!("The Coordinator shows a dialog on its screen that waits for your answer: it is in herdr's tab \"{}\".", crate::coordinator::place::COORDINATOR_TAB), ..Default::default() }); return (parts, false, false, true); }
    (with_mine(parts, &v.sent), false, !v.sent.is_empty(), false)
}
