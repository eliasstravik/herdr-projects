//! The words a call sends: the voice agent's own clear instruction or question for the agent it talks to, written from
//! what you asked of that agent, with one short line that says it came from a voice call; and the four actions that
//! need your request and your yes, by the words that name them (confirm.rs checks them against what was said).
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
/// The message as it goes to the agent: the voice agent's own instruction or question for that agent (written from
/// what you asked of it, in your intent and key wording), and one short line that says it came from a voice call and
/// what it authorizes. The board cannot read a sentence for what it orders (a question may name an action, an order
/// may hide in many words), so it never tries: the line says it plainly. With no action (`none`): that no approve,
/// merge, stop or delete was asked for and confirmed, so the agent does none because of it. With one you asked for and
/// confirmed on the call (confirm.rs): that one, for the tasks the question you said yes to named (`tasks`), and no
/// other. What you said is not pasted in: the board keeps it to check an action against.
pub fn message(said: &str, action: &str, tasks: &[String]) -> String {
    let none = "No approve, merge, stop or delete was asked for and confirmed with this message: do none of those because of it.";
    let what = if action.is_empty() || action == "none" { none.to_string() } else {
        let on = if tasks.is_empty() { String::new() } else { format!(" for {}", tasks.join(", ")) };
        format!("The human asked for this {action}{on} on the call and confirmed it; only that: no other approve, merge, stop or delete was confirmed, so do none of those because of it.")
    };
    format!("{}\n\n(Via voice call: answer in one or two short plain sentences first, they are read aloud. {what})", squash(said))
}
