//! The words a call sends: your own, word for word, first (what the voice agent made of them only beside them), with one
//! line that says they were spoken. Nothing you said is left out; where your words name an action that was not
//! confirmed (approve, merge, stop, delete), the line says so, so the agent does not take them as a request.
use crate::text::squash;

/// The four actions that need your request and your yes, each by the words that name it (English and Swedish).
pub const ACTIONS: [(&str, &[&str]); 4] = [
    ("approve", &["approve", "approved", "approval", "accept", "godkänn", "godkänna", "godkänd", "godkänt", "godkänner"]),
    ("merge", &["merge", "merged", "merging", "merga", "mergea", "slå ihop", "slå samman", "slår ihop"]),
    ("stop", &["stop", "stopp", "stoppa", "stoppar", "avbryt", "avbryta", "cancel", "kill", "abort"]),
    ("delete", &["delete", "remove", "archive", "radera", "ta bort", "tar bort", "arkivera", "släng", "kasta"]),
];
/// Your words, longest kept with one message (its end: the newest).
const MOST: usize = 1500;
/// The words of a text, lower case, one space between, one at each end: " merge hp 52 ".
pub fn words_of(s: &str) -> String { format!(" {} ", s.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")) }
/// The actions these words name.
pub fn actions_in(s: &str) -> Vec<&'static str> {
    let w = words_of(s);
    ACTIONS.iter().filter(|(_, names)| names.iter().any(|n| w.contains(&format!(" {n} ")))).map(|(a, _)| *a).collect()
}
/// The message as it goes to the agent: everything you said since the last message, as you said it, then a line that
/// says it was spoken (and what the voice agent took it as, when that says something else).
pub fn message(said: &str, heard: &[String]) -> String {
    let said = squash(said);
    let all = heard.iter().map(|h| squash(h)).filter(|h| !h.is_empty()).collect::<Vec<_>>().join(" ");
    let words = if all.chars().count() > MOST { format!("…{}", all.chars().skip(all.chars().count() - MOST).collect::<String>()) } else { all };
    let note = "Answer in one or two short plain sentences first: they are read aloud on the call.";
    if words.is_empty() { return format!("{said}\n\n(Sent from a voice call. {note})"); }
    let carried = actions_in(&said);
    let loose: Vec<&str> = actions_in(&words).into_iter().filter(|a| !carried.contains(a)).collect();
    let warn = if loose.is_empty() { String::new() } else { format!(" Where these words name {}, that is not a request to do it: nothing of the kind was confirmed.", loose.join(" or ")) };
    let same = words_of(&words).contains(words_of(&said).trim()) || words_of(&said).trim().is_empty();
    if same { format!("{words}\n\n(Spoken on a voice call, word for word.{warn} {note})") } else { format!("{words}\n\n(Spoken on a voice call, word for word; the voice assistant took it as: \"{said}\".{warn} {note})") }
}
