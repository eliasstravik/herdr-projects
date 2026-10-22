//! A closing note's open follow-ups, read as a list: the items of its "Still open" and "Also noticed" sections (and
//! of one called "Follow-ups"), each one line with, where the agent wrote it, the repo it belongs to (`[repo: web]`)
//! and whether the change is not complete or usable without it (`(blocks)`). Plain code, no model: a note without such
//! a section has none.
use crate::json::*;

/// The sections whose items are follow-ups, by the words their heading starts with, and the kind each gives.
const SECTIONS: [(&str, &str); 6] = [("still open", "open"), ("also noticed", "noticed"), ("follow-ups", "open"), ("follow ups", "open"), ("followups", "open"), ("open follow-ups", "open")];
/// An item that says there is nothing: its first word, or "nothing" with one of these after it ("nothing else").
const NONE: [&str; 3] = ["none", "n/a", "-"];
const NOTHING_MORE: [&str; 11] = ["else", "worth", "more", "left", "open", "to", "further", "noticed", "found", "outstanding", "new"];
fn says_nothing(words: &str) -> bool {
    let w: Vec<String> = words.split_whitespace().map(|x| x.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '-').to_lowercase()).filter(|x| !x.is_empty()).collect();
    match w.first().map(|s| s.as_str()) { None => true, Some(f) if NONE.contains(&f) => true, Some("nothing") => w.len() == 1 || NOTHING_MORE.contains(&w[1].as_str()), _ => false }
}
/// The longest heading line taken for a section's: a longer line is a sentence that happens to start with its words.
const HEADING_MAX: usize = 60;

/// The heading's own words: no `#`, no bold or italics, no colon at the end.
fn heading_words(line: &str) -> String {
    line.trim().trim_start_matches('#').trim().trim_matches(|c| c == '*' || c == '_').trim().trim_end_matches(':').trim_matches(|c| c == '*' || c == '_').trim_end_matches(':').trim().to_lowercase()
}
/// The section a line opens, and what stands after its colon on the same line (`Also noticed: the README is stale`).
fn section_of(line: &str) -> Option<(&'static str, String)> {
    let raw = line.trim();
    if raw.starts_with(['-', '*', '+']) && !raw.starts_with("**") { return None; }
    let (head, rest) = match raw.find(':') { Some(i) if !raw[..i].trim_matches(|c| c == '*' || c == '_' || c == '#' || c == ' ').is_empty() => (&raw[..i + 1], raw[i + 1..].trim_start_matches('*').trim().to_string()), _ => (raw, String::new()) };
    let words = heading_words(head);
    if words.chars().count() > HEADING_MAX { return None; }
    SECTIONS.iter().find(|(w, _)| words == *w || words.starts_with(&format!("{w} ")) || words.starts_with(&format!("{w} ("))).map(|(_, k)| (*k, rest))
}
/// A list item's text, without its bullet or number.
fn item_of(line: &str) -> Option<String> {
    let l = line.trim_start();
    for b in ["- [ ] ", "- ", "* ", "+ ", "• "] { if let Some(r) = l.strip_prefix(b) { return Some(r.trim().to_string()); } }
    let digits = l.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 { let r = &l[digits..]; if let Some(r) = r.strip_prefix(". ").or_else(|| r.strip_prefix(") ")) { return Some(r.trim().to_string()); } }
    None
}
/// One item as kept: its words, and the repo and the blocking mark taken out of them.
fn entry(kind: &str, raw: &str) -> Option<Value> {
    let mut words = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut repo = String::new();
    if let Some(m) = crate::re!(r"(?i)\[\s*repo\s*:\s*([^\]\s]+)\s*\]").captures(&words) { repo = m[1].trim_matches('`').to_string(); words = words.replace(&m[0], ""); }
    let tag = crate::re!(r"(?i)\(\s*(blocks|blocking)\s*\)|\[\s*(blocks|blocking)\s*\]");
    let blocks = tag.is_match(&words);
    let words = tag.replace_all(&words, "").split_whitespace().collect::<Vec<_>>().join(" ");
    let words = words.trim_matches(|c: char| c == ' ' || c == '·' || c == ',' || c == ';').to_string();
    if words.is_empty() || says_nothing(&words) { return None; }
    let mut o = obj(json!({ "text": words, "kind": kind }));
    if !repo.is_empty() { o.put("repo", repo); }
    if blocks { o.put("blocks", true); }
    Some(Value::Object(o))
}
/// The follow-ups of a closing note, in the order it lists them.
pub fn parse(note: &str) -> Vec<Value> {
    let mut out: Vec<(String, String)> = vec![];
    let mut kind: Option<&str> = None;
    let mut blank = false;
    for line in note.lines() {
        if line.trim().is_empty() { blank = true; continue; }
        if let Some((k, rest)) = section_of(line) { kind = Some(k); blank = false; if !rest.is_empty() { out.push((k.into(), rest)); } continue; }
        let Some(k) = kind else { continue };
        if let Some(item) = item_of(line) { out.push((k.into(), item)); blank = false; continue; }
        // A line under an item, indented and not after a gap, goes on with it; anything else ends the section.
        if !blank && line.starts_with([' ', '\t']) { if let Some(last) = out.last_mut() { last.1.push(' '); last.1.push_str(line.trim()); continue; } }
        kind = None;
    }
    out.iter().filter_map(|(k, t)| entry(k, t)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_both_sections_their_tags_and_nothing_else() {
        let note = "Did the thing.\n\n## Still open\n- Check it on staging (blocks)\n- [repo: web] Show the new field\n  on the settings page\n\n**Also noticed:**\n1. `src/a.rs`: an old comment\n\nThat is all.\n- not an item";
        let f = parse(note);
        assert_eq!(f.len(), 3, "{f:?}");
        assert_eq!((f[0].s("text"), f[0].b("blocks"), f[0].s("kind")), ("Check it on staging", true, "open"));
        assert_eq!((f[1].s("text"), f[1].s("repo")), ("Show the new field on the settings page", "web"));
        assert_eq!((f[2].s("text"), f[2].s("kind")), ("`src/a.rs`: an old comment", "noticed"));
        assert!(parse("Also noticed: none\n\nStill open: nothing").is_empty());
        for none in ["Also noticed: nothing else.", "Also noticed: nothing worth mentioning", "## Still open\n- None — everything is done.", "Still open:\n- Nothing."] { assert!(parse(none).is_empty(), "{none}"); }
        assert_eq!(parse("Still open:\n- Nothing logs the retry yet").len(), 1);
        assert_eq!(parse("Also noticed: the README is stale")[0].s("text"), "the README is stale");
        assert!(parse("Still open is the question of whether this is a sentence long enough to be no heading at all, really.\n- x").is_empty());
    }
}
