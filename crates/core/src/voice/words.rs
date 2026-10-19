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
/// The four actions as a command says them: the words that order it (never "merged", "approval", "stopped").
const COMMANDS: [(&str, &[&str]); 4] = [
    ("approve", &["approve", "accept", "godkänn", "godkänna"]),
    ("merge", &["merge", "merga", "mergea", "slå ihop", "slå samman"]),
    ("stop", &["stop", "stoppa", "avbryt", "avbryta", "cancel", "kill", "abort"]),
    ("delete", &["delete", "remove", "archive", "radera", "ta bort", "arkivera", "släng", "kasta"]),
];
/// Words a question starts with; words that join a second sentence to it; words that make a command word a thing
/// ("the merge", "a failed merge") before it, or after it ("merge conflict", "stop button").
const ASKS: [&str; 23] = ["what", "why", "how", "when", "where", "who", "which", "does", "did", "do", "is", "are", "was", "has", "have", "vad", "varför", "hur", "när", "vem", "vilken", "vilket", "har"];
const JOINS: [&str; 7] = ["and", "then", "also", "och", "sen", "sedan", "också"];
const BEFORE: [&str; 22] = ["the", "a", "an", "this", "that", "its", "their", "his", "her", "my", "your", "our", "of", "last", "first", "next", "failed", "den", "det", "en", "ett", "sista"];
const AFTER: [&str; 20] = ["failed", "fail", "fails", "failure", "conflict", "conflicts", "error", "errors", "status", "request", "requests", "check", "checks", "log", "logs", "step", "button", "misslyckades", "konflikt", "problem"];
/// The actions these words order done: a command ("merge HP-52", "can you approve it?"), not a question about one
/// ("why did the merge of HP-52 fail?") or a mention ("the approval failed", "look into the failed merge"). A
/// question that goes on into a second sentence ("why did it fail, and then merge it") is read for commands too.
pub fn commands_in(s: &str) -> Vec<&'static str> {
    let w: Vec<String> = words_of(s).split_whitespace().map(String::from).collect();
    let joined = w.iter().any(|x| JOINS.contains(&x.as_str()));
    if w.first().map(|f| ASKS.contains(&f.as_str())).unwrap_or(false) && !joined { return vec![]; }
    COMMANDS.iter().filter(|(_, forms)| forms.iter().any(|form| {
        let f: Vec<&str> = form.split(' ').collect();
        (0..w.len()).any(|i| w.len() >= i + f.len() && f.iter().enumerate().all(|(k, x)| w[i + k] == *x)
            && !(i > 0 && BEFORE.contains(&w[i - 1].as_str())) && !w.get(i + f.len()).map(|n| AFTER.contains(&n.as_str())).unwrap_or(false))
    })).map(|(a, _)| *a).collect()
}
/// The message as it goes to the agent: the voice agent's own instruction or question for that agent (written from
/// what you asked of it, in your intent and key wording), and one short line that says it came from a voice call.
/// What you said is not pasted in: the board keeps it to check an action against (confirm.rs).
pub fn message(said: &str) -> String {
    format!("{}\n\n(Via voice call: answer in one or two short plain sentences first, they are read aloud.)", squash(said))
}
