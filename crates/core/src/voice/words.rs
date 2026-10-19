//! The words a call sends: the voice agent's own clear instruction or question for the agent it talks to, written from
//! what you asked of that agent, with nothing added and the voice's expression tags taken out; and the four actions
//! that need your request and your yes, by the words that name them (confirm.rs checks them against what was said).
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
/// The message as it goes to the agent and shows in the chat: the voice agent's own instruction or question for that
/// agent (written from what you asked of it, in your intent and key wording), and nothing else. What it may do is the
/// board's to check (confirm.rs), never words added to it; what you said stays on the board for that check.
pub fn message(said: &str) -> String { squash(&untagged(said)) }
/// Words without the voice's expression tags ("[neutral]", "[laughs softly]"): they are for how it speaks, never shown
/// or sent. A tag is a short run of lower-case words in square brackets.
pub fn untagged(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('[') {
        let tag = rest[i + 1..].find(']').map(|j| &rest[i + 1..i + 1 + j]);
        match tag {
            Some(t) if !t.is_empty() && t.len() <= 40 && t.chars().all(|c| c.is_lowercase() || c == ' ' || c == '-') => { out.push_str(&rest[..i]); rest = &rest[i + t.len() + 2..]; }
            _ => { out.push_str(&rest[..=i]); rest = &rest[i + 1..]; }
        }
    }
    out.push_str(rest);
    squash(&out)
}
