//! A conversation's text as the lines of a box: each thing said under a line that names who said it, wrapped to the
//! box, with markdown read as text (a list item's next lines stand under its words, code keeps its lines, the marks
//! of a headline, of bold and of a code block are left out). A picture's chip in what was sent (`[Image 1]`) is in the
//! accent, as in the box.
use super::talk::Part;
use crate::shown::ink::{Ink, Inked};

fn wrap(text: &str, width: usize, lead: &str, hang: &str) -> Vec<Inked> {
    let rows = Inked::plain(text).wrap(width.saturating_sub(crate::text::width(lead)).max(1));
    rows.into_iter().enumerate().map(|(n, l)| Inked::plain(if n > 0 { hang } else { lead }).add(l)).collect()
}
fn clean(s: &str) -> String { crate::shown::words::clean(s) }
/// A line of what was sent with each picture's chip in it in the accent.
fn chips(l: Inked) -> Inked {
    let re = crate::re!(r"\[Image \d+\]");
    if !re.is_match(&l.text()) { return l; }
    let mut out = Inked::new();
    for p in &l.0 {
        let mut at = 0;
        let mut piece = |t: &str, c: Ink| if !t.is_empty() { out.0.push(crate::shown::ink::Piece { t: t.into(), c, role: None, ..p.clone() }); };
        for m in re.find_iter(&p.t) { piece(&p.t[at..m.start()], p.c); piece(m.as_str(), Ink::Accent); at = m.end(); }
        piece(&p.t[at..], p.c);
    }
    if let Some(r) = l.role_of() { out = out.role(r); }
    out
}
/// One text, wrapped to `width`. `raw`: a screen's lines, kept as they are.
pub fn prose_lines(text: &str, width: usize, raw: bool) -> Vec<Inked> {
    let mut out = vec![];
    let mut code = false;
    let bold = crate::re!(r"\*\*([^*\n]+)\*\*");
    let head = crate::re!(r"^(\s*)#{1,6}\s+");
    let lead_re = crate::re!(r"^\s*(?:[-*+•]|\d+[.)])\s+|^\s*>\s?|^\s+");
    for line in text.split('\n') {
        if raw { out.extend(wrap(&clean(line), width, "", "")); continue; }
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") { code = !code; continue; }
        if code { out.extend(wrap(&clean(&line.replace('\t', "  ")), width, "  ", "  ")); continue; }
        let l = clean(line);
        let l = bold.replace_all(&l, "$1").into_owned();
        let l = head.replace(&l, "$1").into_owned();
        let lead = lead_re.find(&l).map(|m| m.as_str().to_string()).unwrap_or_default();
        out.extend(wrap(&l[lead.len()..], width, &lead, &" ".repeat(crate::text::width(&lead))));
    }
    out
}
/// The parts of a chat (oldest first) as lines: an empty line between two, and the line that names who said it first
/// (not for your words, nor for what the one the box talks to said).
/// `paint(by, line, part)` colours a line. Returns the lines and the one each part starts on.
pub fn part_lines(parts: &[Part], width: usize, paint: &dyn Fn(bool, Inked, &Part) -> Inked) -> (Vec<Inked>, Vec<usize>) {
    let mut lines = vec![];
    let mut starts = vec![];
    for (n, p) in parts.iter().enumerate() {
        if n > 0 { lines.push(Inked::new()); }
        starts.push(lines.len());
        // Who said it, but not for you nor for the one the box talks to: your words are as bright as you type them,
        // theirs grey (view/chat.rs), and its edge names them. A reviewer, autopilot and the board are named.
        let own = (!p.sent && !p.raw && p.tone.is_empty() && p.by != "Reviewer") || p.by == "You";
        if !p.by.is_empty() && !own { for l in wrap(&format!("{}:", clean(&p.by)), width, "", "") { lines.push(paint(true, l, p)); } }
        for l in prose_lines(&p.text, width, p.raw) { let l = paint(false, l, p); lines.push(if p.sent { chips(l) } else { l }); }
    }
    (lines, starts)
}
