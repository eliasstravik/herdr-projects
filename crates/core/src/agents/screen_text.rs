//! The words on a program's screen, for a program whose saved conversation the board cannot read. What a terminal
//! program draws is told from what it says by its shape alone: a line that is mostly drawing (a rule, a box's edge, a
//! logo made of dots or blocks, a bar) holds few letters among its marks, and is left out; a box's edges are taken off.
//! And what the agent said last, without the program's input box and the lines it draws under it.
const EDGE: &[char] = &['│', '┃', '|', '╭', '╮', '╰', '╯', '┌', '┐', '└', '┘', '─', '━', '═'];
fn edge_trim(s: &str) -> &str { s.trim_matches(|c: char| c.is_whitespace() || EDGE.contains(&c)) }
fn drawn(line: &str) -> bool {
    let marks: Vec<char> = line.chars().filter(|c| !c.is_whitespace()).collect();
    let words = marks.iter().filter(|c| c.is_alphanumeric()).count();
    !marks.is_empty() && words * 2 < marks.len()
}
pub fn screen_text(screen: &str) -> String {
    let mut out: Vec<String> = vec![];
    for raw in screen.split('\n') {
        let line = edge_trim(raw);
        if !raw.trim().is_empty() && (line.is_empty() || drawn(line)) { continue; }
        if line.is_empty() && out.last().map(|l| l.is_empty()).unwrap_or(true) { continue; } // one empty line between blocks
        if line.is_empty() { out.push(String::new()); continue; }
        let mut s = raw.trim_start();
        if let Some(r) = s.strip_prefix(['│', '┃', '|']) { s = r.strip_prefix(' ').unwrap_or(r); } else { s = raw; }
        let s = s.trim_end();
        let s = s.strip_suffix(['│', '┃', '|']).map(|x| x.trim_end()).unwrap_or(s);
        out.push(s.to_string());
    }
    while out.last().map(|l| l.is_empty()).unwrap_or(false) { out.pop(); }
    out.join("\n")
}
/// A rule across the screen: a line between two parts, or a box's top or bottom edge.
fn rule(line: &str) -> bool {
    let s = line.trim().trim_matches(['╭', '╮', '╰', '╯', '┌', '┐', '└', '┘']);
    s.chars().count() >= 8 && s.chars().all(|c| ['─', '━', '═'].contains(&c))
}
/// The words an agent left on its screen, for a turn that ended with nothing else: what stands above its program's
/// input box. The box is its prompt mark near the end of the screen (Claude Code's `❯`, Codex's `›`, or `>` just under
/// a rule), with the rule over it; what is drawn under it (a mode line, a status line: Claude Code's "⏵⏵ bypass
/// permissions on") is the program's, never the agent's words.
pub fn agent_words(screen: &str) -> String {
    let lines: Vec<&str> = screen.split('\n').collect();
    let below = |i: usize| lines[i + 1..].iter().filter(|l| !l.trim().is_empty()).count();
    let above = |i: usize| (0..i).rev().find(|j| !lines[*j].trim().is_empty());
    let ruled = |i: usize| above(i).filter(|j| rule(lines[*j]));
    let input = (0..lines.len()).rev().take_while(|i| below(*i) <= 8).find(|i| {
        let l = edge_trim(lines[*i]);
        l.starts_with(['❯', '›']) || (l.starts_with('>') && ruled(*i).is_some())
    });
    match input {
        Some(i) => screen_text(&lines[..ruled(i).unwrap_or(i)].join("\n")),
        None => screen_text(screen),
    }
}
