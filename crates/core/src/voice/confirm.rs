//! The check before an action goes out: approving, merging, stopping and removing go to an agent only when you asked
//! for that, the voice agent asked you back ("Merge HP-52, right?"), and the last thing you said is a plain yes to it.
//! It reads the call as it was said since the last message (`talk`: your lines and the voice agent's, in order), so
//! the voice agent's own word that you said yes is never enough. A question ("what does merge mean?"), a no ("no, do
//! not do it") or a yes to something else ("yes, explain it") is no request and no yes; a yes to HP-52 is no yes to
//! HP-99, also when the yes names HP-99 itself ("yes, merge HP-99"), and a yes that names another action is no yes to
//! this one. A no takes back every request before it, and a question that offers something else ("would you like an
//! explanation of merge?") is no question to do it.
use super::words::{actions_in, words_of, ACTIONS};

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict { Send, Ask(String), Refuse(String) }
const YES: [&str; 17] = ["yes", "yeah", "yep", "yup", "sure", "ok", "okay", "correct", "right", "absolutely", "ja", "japp", "jo", "jajamän", "absolut", "kör", "stämmer"];
/// Words a yes may carry besides: please, go ahead, do it (English and Swedish).
const ALSO: [&str; 26] = ["please", "do", "it", "go", "ahead", "that", "thanks", "thank", "you", "just", "now", "the", "one", "exactly", "yes", "tack", "gärna", "gör", "det", "bara", "nu", "precis", "du", "kan", "så", "kör"];
const NO: [&str; 14] = ["no", "not", "never", "wait", "nope", "don", "dont", "won", "nej", "inte", "vänta", "nä", "ej", "aldrig"];
/// Words a question starts with ("can you merge it?" is a request, so not these), and words that ask what a word means.
const ASKS: [&str; 19] = ["what", "why", "how", "when", "where", "who", "which", "does", "did", "is", "are", "was", "vad", "varför", "hur", "när", "vem", "vilken", "vilket"];
const MEANS: [&str; 4] = ["mean", "means", "meaning", "betyder"];
/// Words of a question that offers something other than doing it: an explanation, more, the status (English, Swedish).
const OFFERS: [&str; 30] = ["explain", "explanation", "explaining", "describe", "tell", "more", "detail", "details", "info", "information", "why", "mean", "meaning", "summary", "summarize", "status", "read", "know", "learn", "instead", "förklara", "förklaring", "berätta", "mer", "detaljer", "varför", "betyder", "sammanfatta", "läsa", "veta"];
fn has(w: &str, list: &[&str]) -> bool { list.iter().any(|x| w.contains(&format!(" {x} "))) }
/// A task's tag in words ("HP-52", "hp 52"), as one word: "HP52".
pub fn tags_in(s: &str) -> Vec<String> { crate::re!(r"(?i)\b([a-z]{2,5})[- ]?(\d+)\b").captures_iter(s).map(|c| format!("{}{}", c[1].to_uppercase(), &c[2])).collect() }
/// A question about it, not a request to do it: starts with a question word, or asks what it means.
fn is_question(s: &str) -> bool { let w = words_of(s); w.split_whitespace().next().map(|f| ASKS.contains(&f)).unwrap_or(false) || has(&w, &MEANS) }
fn negated(s: &str) -> bool { has(&words_of(s), &NO) }
/// A plain yes: a yes word, no no, and nothing else but words a yes carries, the action, and the task's tag.
fn plain_yes(s: &str) -> bool {
    let w = words_of(s);
    if !has(&w, &YES) || negated(s) { return false; }
    let action_words: Vec<&str> = ACTIONS.iter().flat_map(|(_, n)| n.iter().flat_map(|x| x.split(' '))).collect();
    let tags: Vec<String> = tags_in(s).iter().map(|t| t.to_lowercase()).collect();
    w.split_whitespace().all(|x| YES.contains(&x) || ALSO.contains(&x) || action_words.contains(&x) || x.chars().all(|c| c.is_ascii_digit()) || tags.iter().any(|t| t.starts_with(x)))
}
/// May `said` go to the agent? `action`: the action the voice agent says it carries ("none", or approve, merge, stop,
/// delete). `talk`: (who, words) since the last message, "you" for you and "voice" for the voice agent. A message with
/// none always goes: the line under it tells the agent it authorizes none of the four (words.rs: message). A named one
/// goes only as checked here, and authorizes that one alone.
pub fn check(said: &str, action: &str, talk: &[(String, String)], confirmed: bool) -> Verdict {
    let Some(named) = ACTIONS.iter().map(|(a, _)| *a).find(|a| *a == action) else { return Verdict::Send };
    let wanted = vec![named];
    let you = |i: usize| talk[i].0 == "you";
    // A no of yours takes back every request before it.
    let after_no = (0..talk.len()).rev().find(|&i| you(i) && negated(&talk[i].1)).map(|i| i + 1).unwrap_or(0);
    // your request: words of yours since your last no that name it, and are no question and no no
    let asked_at = |a: &str| (after_no..talk.len()).filter(|&i| you(i) && actions_in(&talk[i].1).contains(&a) && !is_question(&talk[i].1) && !negated(&talk[i].1)).collect::<Vec<_>>();
    if let Some(a) = wanted.iter().find(|a| asked_at(a).is_empty()) {
        return Verdict::Refuse(format!("Not sent: it asks to {a}, and the human did not ask for that. Send only what they asked for."));
    }
    let ask = Verdict::Ask(format!("Not sent yet: to {} needs a clear yes. Ask the human a short question that names it and the task (\"{} HP-52, right?\"), and after their yes send again with confirmed true.", wanted.join(" and "), wanted[0]));
    if !confirmed { return ask; }
    let Some(yes) = (0..talk.len()).rev().find(|&i| you(i)) else { return ask };
    if !plain_yes(&talk[yes].1) { return ask; }
    // the voice agent's question right before your yes: it names the action, and every task the message names
    let Some(q) = (0..yes).rev().find(|&i| !you(i)) else { return ask };
    let question = &talk[q].1;
    // a question to do it: it names the action, and offers nothing else (an explanation, more, the status)
    if !question.contains('?') || wanted.iter().any(|a| !actions_in(question).contains(a)) || has(&words_of(question), &OFFERS) || negated(question) { return ask; }
    let (in_q, in_said) = (tags_in(question), tags_in(said));
    if let Some(t) = in_said.iter().find(|t| !in_q.contains(t)) {
        return Verdict::Refuse(format!("Not sent: the human said yes to \"{question}\", which is not about {t}. Ask again about the task they mean."));
    }
    // the yes itself: a task or an action it names must be the ones asked about. Any number counts as a task's
    // ("#99", "99", "HP 99"): every number the yes or the message says must be one the question said.
    let yes_words = &talk[yes].1;
    let numbers = |s: &str| crate::re!(r"\d+").find_iter(s).map(|m| m.as_str().trim_start_matches('0').to_string()).collect::<Vec<_>>();
    let in_question = numbers(question);
    if let Some(n) = numbers(yes_words).into_iter().chain(numbers(said)).find(|n| !in_question.contains(n)) {
        return Verdict::Ask(format!("Not sent: {n} was not in the question the human said yes to. Ask again which task they mean."));
    }
    if let Some(t) = tags_in(yes_words).into_iter().find(|t| !in_q.contains(t) || !in_said.contains(t)) {
        return Verdict::Ask(format!("Not sent: the human's yes names {t}, and the question was not about that. Ask again which task they mean."));
    }
    if actions_in(yes_words).iter().any(|a| !wanted.contains(a)) { return ask; }
    if wanted.iter().any(|a| !asked_at(a).iter().any(|&i| i < q)) { return ask; }
    Verdict::Send
}
/// The tasks the question you said yes to named ("HP52"), for the line under a confirmed action.
pub fn confirmed_tasks(talk: &[(String, String)]) -> Vec<String> {
    let Some(yes) = (0..talk.len()).rev().find(|&i| talk[i].0 == "you") else { return vec![] };
    let dash = |t: String| { let at = t.find(|c: char| c.is_ascii_digit()).unwrap_or(t.len()); format!("{}-{}", &t[..at], &t[at..]) };
    (0..yes).rev().find(|&i| talk[i].0 != "you").map(|q| tags_in(&talk[q].1).into_iter().map(dash).collect()).unwrap_or_default()
}
