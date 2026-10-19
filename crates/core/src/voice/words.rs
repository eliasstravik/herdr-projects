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
/// Words that may stand before an order in its sentence ("please merge", "do merge", "can you merge", "go ahead and
/// merge", "kan du godkänna").
const LEAD: [&str; 26] = ["please", "do", "now", "then", "just", "also", "so", "ok", "okay", "and", "go", "ahead", "kindly", "can", "could", "would", "will", "you", "kan", "du", "snälla", "och", "sen", "gärna", "nu", "bara"];
/// The actions these words order done, sentence by sentence (and clause by clause): an order starts its sentence,
/// after at most a few words of politeness ("Merge HP-52", "Do merge HP-52", "Why did it fail? Merge HP-52.", "Can
/// you approve it?"). A question about one or a mention ("why did the merge fail?", "explain whether we should merge")
/// orders nothing.
pub fn commands_in(s: &str) -> Vec<&'static str> {
    let low = s.to_lowercase();
    let clauses: Vec<String> = crate::re!(r"[.?!;:,\n]|\band then\b|\bthen\b|\boch sen\b").split(&low).map(|c| words_of(c)).collect();
    COMMANDS.iter().filter(|(_, forms)| clauses.iter().any(|c| {
        let w: Vec<&str> = c.split_whitespace().collect();
        let start = w.iter().take_while(|x| LEAD.contains(x)).count();
        forms.iter().any(|form| { let f: Vec<&str> = form.split(' ').collect(); w.len() >= start + f.len() && f.iter().enumerate().all(|(k, x)| w[start + k] == *x) })
    })).map(|(a, _)| *a).collect()
}
/// The message as it goes to the agent: the voice agent's own instruction or question for that agent (written from
/// what you asked of it, in your intent and key wording), and one short line that says it came from a voice call, and
/// whether it carries an action you asked for and confirmed (`action`) or none: then the agent is told to do none
/// because of it. What you said is not pasted in: the board keeps it to check an action against (confirm.rs).
pub fn message(said: &str, action: &str) -> String {
    let what = if action.is_empty() || action == "none" { "No approve, merge, stop or delete was confirmed with it: do none of those because of it.".to_string() } else { format!("The human asked for this {action} on the call and confirmed it.") };
    format!("{}\n\n(Via voice call: answer in one or two short plain sentences first, they are read aloud. {what})", squash(said))
}
