//! The words a call sends: the voice agent's own clear instruction or question for the agent it talks to, written from
//! what you asked of that agent, with the voice's expression tags taken out and nothing added but, where its words may
//! order an action nobody confirmed, one line that says so; a confirmed action as the board writes it ("Merge HP-52.");
//! and the four actions that need your request and your yes, by the words that name them (confirm.rs checks them).
use crate::text::squash;

/// The four actions that need your request and your yes, each by the words that name it (English and Swedish).
pub const ACTIONS: [(&str, &[&str]); 4] = [
    ("approve", &["approve", "approved", "approval", "accept", "godkänn", "godkänna", "godkänd", "godkänt", "godkänner"]),
    ("merge", &["merge", "merged", "merging", "merga", "mergea", "slå ihop", "slå samman", "slår ihop"]),
    ("stop", &["stop", "stopp", "stoppa", "stoppar", "avbryt", "avbryta", "cancel", "kill", "abort"]),
    ("delete", &["delete", "remove", "archive", "radera", "ta bort", "tar bort", "arkivera", "släng", "kasta"]),
];
/// The words of a text, lower case, one space between, one at each end: " merge hp 52 ".
pub fn words_of(s: &str) -> String { format!(" {} ", s.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")) }
/// The actions these words name.
pub fn actions_in(s: &str) -> Vec<&'static str> {
    let w = words_of(s);
    ACTIONS.iter().filter(|(_, names)| names.iter().any(|n| w.contains(&format!(" {n} ")))).map(|(a, _)| *a).collect()
}
/// Other words that may order one of the four (ship it, get rid of it, kill it): a message with them and no action
/// confirmed carries the one line that says so (`message`), and a confirmed action's message may not hold them for
/// another action (confirm.rs). Generous: a word too many only adds that line.
const ALSO_ORDERS: [(&str, &[&str]); 4] = [
    ("approve", &["lgtm", "sign off", "signed off", "finish", "finish it", "ok it", "okay it", "greenlight", "green light", "godkänd den", "klarmarkera"]),
    ("merge", &["ship", "ship it", "land", "land it", "release", "go live", "push it", "integrate", "combine", "fold", "into main", "to main", "skeppa", "släpp", "landa", "integrera"]),
    ("stop", &["halt", "terminate", "pause", "interrupt", "end it", "shut down", "standstill", "freeze", "suspend", "pausa", "avsluta", "stäng av", "frys"]),
    ("delete", &["erase", "wipe", "discard", "drop", "trash", "close", "get rid of", "throw away", "destroy", "purge", "rensa", "stäng", "töm", "förstör"]),
];
/// The actions these words name or may order, by the four's own words and the others (ship it, get rid of it).
pub fn orders_in(s: &str) -> Vec<&'static str> {
    let w = words_of(s);
    let by = |l: &[(&'static str, &[&str])]| l.iter().filter(|(_, names)| names.iter().any(|n| w.contains(&format!(" {n} ")))).map(|(a, _)| *a).collect::<Vec<_>>();
    let (mut all, more) = (by(&ACTIONS), by(&ALSO_ORDERS));
    for a in more { if !all.contains(&a) { all.push(a); } }
    all
}
/// Under a message with no action confirmed whose words name or may order one of the four: this one line, and only
/// there. Every other message goes as the voice agent wrote it.
pub const NOT_CONFIRMED: &str = "(Voice call: nothing was confirmed, so ask the human before you approve, merge, stop or delete.)";
/// The message as it goes to the agent and shows in the chat: the voice agent's own instruction or question for that
/// agent (written from what you asked of it, in your intent and key wording). A confirmed action (`action`, checked by
/// confirm.rs) goes as written; a message with none gets the one line `NOT_CONFIRMED` only when its words name or may
/// order one of the four (a question about a merge, or an order the board cannot tell from one). What you said stays
/// on the board for the check, never pasted in.
pub fn message(said: &str, action: &str) -> String {
    let text = squash(&untagged(said));
    let none = action.is_empty() || action == "none";
    if none && !orders_in(&text).is_empty() { format!("{text}\n\n{NOT_CONFIRMED}") } else { text }
}
/// A confirmed action as the board writes it, the action and its tasks alone ("Merge HP-52."), never the voice agent's
/// words: whatever else they held (another action, in any words) is not confirmed and does not go with it. With no
/// task named (`tasks` empty), it is for the task whose agent the call talks to.
pub fn confirmed(action: &str, tasks: &[String]) -> String {
    let verb = ACTIONS.iter().find(|(a, _)| *a == action).map(|(a, _)| { let mut c = a.chars(); c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default() }).unwrap_or_default();
    if tasks.is_empty() { format!("{verb} this task.") } else { format!("{verb} {}.", tasks.join(" and ")) }
}
/// Words without the voice's expression tags ("[neutral]", "[laughs softly]"): they are for how it speaks, never shown
/// or sent. Tags are short lower-case words in square brackets that start a sentence; the last of them is one only
/// when a sentence of its own (a capital letter) or nothing follows: "[neutral] One thing waits." So "items[index]",
/// "the [voice] settings", "section: [voice] should become [calls]" and "[dependencies] must stay" stay as they are.
pub fn untagged(s: &str) -> String {
    let tag = crate::re!(r"^\s*\[\p{Ll}[\p{Ll} -]{0,39}\]\s*");
    let end = crate::re!(r"[.!?…]\s+");
    let (mut out, mut rest) = (String::new(), s);
    loop {
        // a tag before text of your own in brackets ("[neutral] [dependencies] must"): only that text stays
        while let Some(m) = tag.find(rest) {
            let after = &rest[m.end()..];
            if after.is_empty() || after.starts_with('[') || after.chars().next().is_some_and(char::is_uppercase) { rest = after; } else { break; }
        }
        match end.find(rest) { Some(m) => { out.push_str(&rest[..m.end()]); rest = &rest[m.end()..]; } None => { out.push_str(rest); break; } }
    }
    squash(&out)
}
