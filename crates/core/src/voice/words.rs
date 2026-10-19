//! The words a call sends: your own, word for word, first (what the voice agent made of them only beside them), with one
//! line that says they were spoken; and never an action you did not ask for. Approving, merging, stopping and
//! removing go only on your clear spoken request and after a short yes: the voice agent asks, and this checks.
use crate::text::squash;

/// Your words kept with one message: the last few things you said since the last one sent.
const KEEP: usize = 3;
/// The four actions that need your request and your yes, each by the words that name it (English and Swedish).
pub const ACTIONS: [(&str, &[&str]); 4] = [
    ("approve", &["approve", "approved", "approval", "accept", "godkänn", "godkänna", "godkänd", "godkänt", "godkänner"]),
    ("merge", &["merge", "merged", "merging", "merga", "mergea", "slå ihop", "slå samman", "slår ihop"]),
    ("stop", &["stop", "stopp", "stoppa", "stoppar", "avbryt", "avbryta", "cancel", "kill", "abort"]),
    ("delete", &["delete", "remove", "archive", "radera", "ta bort", "tar bort", "arkivera", "släng", "kasta"]),
];
fn words_of(s: &str) -> String { format!(" {} ", s.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")) }
/// The actions these words name.
pub fn actions_in(s: &str) -> Vec<&'static str> {
    let w = words_of(s);
    ACTIONS.iter().filter(|(_, names)| names.iter().any(|n| w.contains(&format!(" {n} ")))).map(|(a, _)| *a).collect()
}
/// Words that say yes, in English and Swedish.
const YES: [&str; 18] = ["yes", "yeah", "yep", "sure", "ok", "okay", "correct", "right", "go", "do it", "please do", "ja", "japp", "jo", "absolut", "kör", "gör det", "stämmer"];
fn says_yes(s: &str) -> bool { let w = words_of(s); YES.iter().any(|y| w.contains(&format!(" {y} "))) }
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict { Send, Ask(String), Refuse(String) }
/// May `said` go to the agent? An action in it must be one your own words asked for, and the last thing you said must
/// be a yes (the voice agent's word that you said yes is not enough).
pub fn check(said: &str, heard: &[String], confirmed: bool) -> Verdict {
    let asked = actions_in(&heard.join(" "));
    let wanted = actions_in(said);
    if let Some(a) = wanted.iter().find(|a| !asked.contains(a)) {
        return Verdict::Refuse(format!("Not sent: it asks to {a}, and the human did not ask for that. Send only what they asked for."));
    }
    if !wanted.is_empty() && !(confirmed && heard.last().map(|h| says_yes(h)).unwrap_or(false)) {
        return Verdict::Ask(format!("Not sent yet: to {} needs a clear yes. Ask in a few words (\"{} it, right?\"), then send again with confirmed true.", wanted.join(" and "), wanted[0]));
    }
    Verdict::Send
}
/// The message as it goes to the agent: your words as you said them, then a line that says they were spoken.
/// What you said that names an action this message does not carry stays out of it: "merge it… no, wait" is not sent
/// along with a question.
pub fn message(said: &str, heard: &[String]) -> String {
    let said = squash(said);
    let carried = actions_in(&said);
    let mine: Vec<String> = heard.iter().rev().take(KEEP).rev().map(|h| squash(h)).filter(|h| !h.is_empty() && actions_in(h).iter().all(|a| carried.contains(a))).collect();
    let note = "Answer in one or two short plain sentences first: they are read aloud on the call.";
    if mine.is_empty() { return format!("{said}\n\n(Sent from a voice call. {note})"); }
    let words = mine.join(" ");
    let same = words_of(&words).contains(words_of(&said).trim()) || words_of(&said).trim().is_empty();
    if same { format!("{words}\n\n(Spoken on a voice call, word for word. {note})") } else { format!("{words}\n\n(Spoken on a voice call, word for word; the voice assistant took it as: \"{said}\". {note})") }
}
