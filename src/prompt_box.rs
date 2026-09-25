//! Is an agent's input box empty? Read from the pane's screen (`agent read
//! --format ansi`), so it works for every harness without hooks: each
//! supported kind has its own anchor for the box, and text drawn dim, a known
//! placeholder, or a harness-drawn cursor over a placeholder does not count as
//! typed. Anything the check cannot place is `Unknown`, and the ticker treats
//! that like a draft: it never types into a box it cannot see is empty.

/// What the input box holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Draft {
    Empty,
    /// Someone typed text that was not submitted.
    Typed,
    /// No input box was found on the screen (a menu or dialog, a scrolled
    /// view, a layout this binary does not know, an unsupported kind).
    Unknown,
}

/// Placeholders drawn in a normal (not dim) colour, per kind. Only strings
/// nobody types by accident.
fn placeholders(kind: &str) -> &'static [&'static str] {
    match kind {
        "gemini" => &["Type your message or @path/to/file"],
        "opencode" => &["Ask anything…"],
        _ => &[],
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Cell {
    ch: char,
    dim: bool,
    reverse: bool,
    bg: String,
}

type Line = Vec<Cell>;

fn text(line: &[Cell]) -> String {
    line.iter().map(|c| c.ch).collect()
}

/// Screen text with SGR styling into cells. Other escape sequences are
/// skipped; only dim, reverse and the background colour are kept.
fn parse(screen: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut line = Line::new();
    let (mut dim, mut reverse, mut bg) = (false, false, String::new());
    let mut chars = screen.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\u{1b}' => {
                if chars.peek() != Some(&'[') {
                    chars.next();
                    continue;
                }
                chars.next();
                let mut params = String::new();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        last = c;
                        break;
                    }
                    params.push(c);
                }
                if last == 'm' {
                    sgr(&params, &mut dim, &mut reverse, &mut bg);
                }
            }
            '\n' => lines.push(std::mem::take(&mut line)),
            '\r' => {}
            c if c.is_control() => {}
            c => line.push(Cell { ch: c, dim, reverse, bg: bg.clone() }),
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn sgr(params: &str, dim: &mut bool, reverse: &mut bool, bg: &mut String) {
    let codes: Vec<&str> = if params.is_empty() { vec!["0"] } else { params.split(';').collect() };
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            "0" | "" => {
                *dim = false;
                *reverse = false;
                bg.clear();
            }
            "2" => *dim = true,
            "22" => *dim = false,
            "7" => *reverse = true,
            "27" => *reverse = false,
            "49" => bg.clear(),
            code @ ("38" | "48") => {
                let take = match codes.get(i + 1) {
                    Some(&"5") => 2,
                    Some(&"2") => 4,
                    _ => 0,
                };
                if code == "48" {
                    *bg = codes[i + 1..(i + 1 + take).min(codes.len())].join(";");
                }
                i += take;
            }
            code if code.len() == 2 && code.starts_with('4') => *bg = code.to_string(),
            code if code.len() == 3 && code.starts_with("10") => *bg = code.to_string(),
            _ => {}
        }
        i += 1;
    }
}

/// The cells after the first `glyph` on the line (the anchor itself dropped).
fn after(line: &[Cell], glyph: char) -> Option<Line> {
    let at = line.iter().position(|c| c.ch == glyph)?;
    Some(line[at + 1..].to_vec())
}

fn trimmed_starts(line: &[Cell], prefix: &str) -> bool {
    text(line).trim_start().starts_with(prefix)
}

/// A horizontal rule: a line of box-drawing `─` only.
fn is_rule(line: &[Cell]) -> bool {
    let t = text(line);
    let t = t.trim();
    t.chars().count() >= 10 && t.chars().all(|c| c == '─')
}

/// The typed text in a box's cells: non-blank, not dim, and not a
/// harness-drawn cursor sitting on a dim placeholder.
fn typed(cells: &[Cell]) -> String {
    let mut out = String::new();
    for (i, c) in cells.iter().enumerate() {
        if c.dim || c.ch.is_whitespace() {
            out.push(' ');
            continue;
        }
        if c.reverse && cells.get(i + 1).is_some_and(|next| next.dim) {
            out.push(' ');
            continue;
        }
        out.push(c.ch);
    }
    out
}

/// The input box's cells, one entry per line, or `None` when it is not on
/// the screen.
fn input_box(kind: &str, lines: &[Line]) -> Option<Vec<Line>> {
    let last = |pred: &dyn Fn(usize, &Line) -> bool| lines.iter().enumerate().rev().find(|(i, l)| pred(*i, l)).map(|(i, _)| i);
    match kind {
        // `❯` right under a rule, continued until the next rule.
        "claude" => {
            let at = last(&|i, l| trimmed_starts(l, "❯") && i > 0 && is_rule(&lines[i - 1]))?;
            let end = (at + 1..lines.len()).find(|&i| is_rule(&lines[i]))?;
            let mut rows = vec![after(&lines[at], '❯')?];
            rows.extend(lines[at + 1..end].iter().cloned());
            Some(rows)
        }
        "codex" => last(&|_, l| trimmed_starts(l, "›")).and_then(|at| after(&lines[at], '›')).map(|row| vec![row]),
        "cursor" => last(&|_, l| trimmed_starts(l, "→")).and_then(|at| after(&lines[at], '→')).map(|row| vec![row]),
        // `│ > ` (or `!` shell mode, `*` yolo mode) in a bordered box.
        "gemini" => {
            let at = last(&|_, l| {
                let t = text(l);
                let t = t.trim_start();
                t.starts_with('│') && matches!(t['│'.len_utf8()..].trim_start().chars().next(), Some('>' | '!' | '*'))
            })?;
            let end = (at + 1..lines.len()).find(|&i| trimmed_starts(&lines[i], "╰"))?;
            let mut rows = Vec::new();
            for (n, line) in lines[at..end].iter().enumerate() {
                let mut row = after(line, '│')?;
                if let Some(close) = row.iter().rposition(|c| c.ch == '│') {
                    row.truncate(close);
                }
                if n == 0 {
                    let prompt = row.iter().position(|c| matches!(c.ch, '>' | '!' | '*'))?;
                    row.drain(..=prompt);
                }
                rows.push(row);
            }
            Some(rows)
        }
        // The `┃` lines above the box's `╹▀▀▀` bottom, less the last one (the
        // mode and model line); only cells on the box's own background count,
        // so the session sidebar to its right is left out.
        "opencode" => {
            let bottom = last(&|_, l| trimmed_starts(l, "╹"))?;
            let top = (0..bottom).rev().take_while(|&i| trimmed_starts(&lines[i], "┃")).last()?;
            if bottom - top < 2 {
                return None;
            }
            let rows = lines[top..bottom - 1]
                .iter()
                .map(|line| {
                    let row = after(line, '┃').unwrap_or_default();
                    let bg = row.first().map(|c| c.bg.clone()).unwrap_or_default();
                    row.into_iter().take_while(|c| c.bg == bg).collect()
                })
                .collect();
            Some(rows)
        }
        // The editor between the last two rules.
        "pi" => {
            let bottom = last(&|_, l| is_rule(l))?;
            let top = (0..bottom).rev().find(|&i| is_rule(&lines[i]))?;
            Some(lines[top + 1..bottom].to_vec())
        }
        _ => None,
    }
}

/// Reads the input box of an agent of `kind` from its screen, as `agent read
/// --source visible --format ansi` prints it.
pub fn check(kind: &str, screen: &str) -> Draft {
    let lines = parse(screen);
    let Some(rows) = input_box(kind, &lines) else {
        return Draft::Unknown;
    };
    let typed: Vec<String> = rows.iter().map(|row| typed(row).trim().to_string()).filter(|t| !t.is_empty()).collect();
    match typed.as_slice() {
        [] => Draft::Empty,
        [only] if placeholders(kind).iter().any(|p| only.starts_with(p)) => Draft::Empty,
        _ => Draft::Typed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/tests/fixtures/prompt_box/{name}.ansi", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn every_supported_kind_tells_an_empty_box_from_a_draft() {
        for kind in ["claude", "codex", "cursor", "gemini", "opencode", "pi"] {
            assert_eq!(check(kind, &fixture(&format!("{kind}-empty"))), Draft::Empty, "{kind} empty");
            assert_eq!(check(kind, &fixture(&format!("{kind}-draft"))), Draft::Typed, "{kind} draft");
        }
        assert_eq!(check("claude", &fixture("claude-empty-after-turn")), Draft::Empty);
        assert_eq!(check("opencode", &fixture("opencode-empty-session")), Draft::Empty);
    }

    #[test]
    fn a_screen_without_the_box_or_an_unknown_kind_is_unknown() {
        assert_eq!(check("copilot", &fixture("claude-empty")), Draft::Unknown);
        assert_eq!(check("claude", "some output\nno box here\n"), Draft::Unknown);
        assert_eq!(check("codex", &fixture("claude-empty")), Draft::Unknown);
    }

    #[test]
    fn menus_and_multi_line_drafts_count_as_typed() {
        // Codex's trust menu uses the same `›` for its selection.
        let menu = "  Do you trust the contents of this directory?\n\u{1b}[1m›\u{1b}[0m 1. Yes, continue\n  2. No, quit\n";
        assert_eq!(check("codex", menu), Draft::Typed);
        let rule = "─".repeat(40);
        let multi = format!("{rule}\n❯ \n  second line typed\n{rule}\n");
        assert_eq!(check("claude", &multi), Draft::Typed);
        let placeholder = format!("{rule}\n❯ \u{1b}[2mTry \"fix lint\"\u{1b}[0m\n{rule}\n");
        assert_eq!(check("claude", &placeholder), Draft::Empty);
        // A transcript line with `❯` that is not under a rule is not the box.
        let transcript = format!("❯ an earlier prompt\n\n{rule}\n❯ \n{rule}\n");
        assert_eq!(check("claude", &transcript), Draft::Empty);
    }

    #[test]
    fn a_real_character_under_a_cursor_is_typed() {
        // One typed character with the harness cursor on it is text, while a
        // cursor on a dim placeholder is not.
        let rule = "─".repeat(40);
        assert_eq!(check("pi", &format!("{rule}\n\u{1b}[7mx\u{1b}[0m   \n{rule}\n")), Draft::Typed);
        assert_eq!(check("cursor", "  \u{1b}[2m→ \u{1b}[0m\u{1b}[7mP\u{1b}[0m\u{1b}[2mlan, search\u{1b}[0m\n"), Draft::Empty);
    }
}
