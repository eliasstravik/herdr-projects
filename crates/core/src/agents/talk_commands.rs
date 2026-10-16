//! A command typed to a program (`/clear`, `/compact`, `/model` …), a stop and a compaction, as a chat shows them: the
//! command as it was typed, then in plain words what it did. Never the program's own markup around it (Claude Code's
//! `<command-name>`, `<local-command-stdout>`, `<local-command-caveat>` …).

/// What a stop says, for every program.
pub const STOPPED: &str = "Stopped: interrupted before it finished.";
/// What a compaction says: asked for (`/compact`), or done by the program itself to make room.
pub const COMPACTED: &str = "Conversation compacted: it goes on from a summary of what was said.";
pub const COMPACTED_ITSELF: &str = "Conversation compacted to make room: it goes on from a summary of what was said.";
/// A text the program wrote for a stop (Claude Code's `[Request interrupted by user]`, `… for tool use]`).
pub fn is_stop(text: &str) -> bool { text.trim_start().starts_with("[Request interrupted by user") }

/// A text that is wholly a program's markup for a command: the command typed, what it printed, or a note of its own.
pub enum Markup { Typed { name: String, line: String }, Printed(String), Note }
/// The words between `<name>` and `</name>`, where the text has them.
fn inside<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let (open, close) = (format!("<{name}>"), format!("</{name}>"));
    let start = text.find(&open)? + open.len();
    Some(text[start..].find(&close).map(|end| &text[start..start + end]).unwrap_or(&text[start..]))
}
/// Claude Code's markup, read. None for anything else: a prompt that only quotes such a tag stays as it was typed.
pub fn claude_markup(text: &str) -> Option<Markup> {
    let t = text.trim();
    if t.starts_with("<command-name>") || t.starts_with("<command-message>") {
        let name = inside(t, "command-name").or_else(|| inside(t, "command-message")).unwrap_or("").trim().trim_start_matches('/').to_string();
        let args = inside(t, "command-args").unwrap_or("").trim();
        let line = if args.is_empty() { format!("/{name}") } else { format!("/{name} {args}") };
        return Some(Markup::Typed { name, line });
    }
    if t.starts_with("<bash-input>") { return Some(Markup::Typed { name: "!".into(), line: format!("! {}", inside(t, "bash-input").unwrap_or("").trim()) }); }
    if t.starts_with("<local-command-stdout>") || t.starts_with("<local-command-stderr>") {
        let out = [inside(t, "local-command-stdout"), inside(t, "local-command-stderr")].into_iter().flatten().map(plain).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n");
        return Some(Markup::Printed(out));
    }
    if ["<local-command-caveat>", "<bash-stdout>", "<bash-stderr>"].iter().any(|p| t.starts_with(p)) { return Some(Markup::Note); }
    None
}
/// Printed words without the colours of a terminal, and "(no content)" as nothing.
fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{1b}' { if it.peek() == Some(&'[') { it.next(); while let Some(x) = it.next() { if x.is_ascii_alphabetic() { break; } } } continue; }
        out.push(c);
    }
    let out = out.trim();
    if out == "(no content)" { String::new() } else { out.to_string() }
}
/// What a command did, said at once: a command whose outcome is known as soon as it was typed. None: what it printed
/// says it (`then`), or nothing does (a shell command, a command of one's own that becomes a prompt).
fn at_once(name: &str) -> Option<&'static str> {
    match name {
        "clear" | "new" | "reset" => Some("Chat reset: a fresh session started."),
        "compact" => Some(COMPACTED),
        "exit" | "quit" => Some("Closed: the program has ended."),
        _ => None,
    }
}
/// What a command did, from what it printed.
fn then(name: &str, printed: &str) -> Option<String> {
    if name == "!" { return None; }
    if name == "model" {
        if let Some(m) = printed.strip_prefix("Set model to ") { return Some(format!("Model switched to {}.", m.trim_end_matches('.'))); }
        if let Some(m) = printed.strip_prefix("Kept model as ") { return Some(format!("Model kept: {}.", m.trim_end_matches('.'))); }
    }
    Some(if printed.is_empty() { format!("/{name}: done.") } else { printed.to_string() })
}
/// One entry of markup, read in order: what the chat adds for it, each `(yours, words)`. `waits` holds the command
/// typed last until what it printed comes; what is printed with no command before it is the program's own, and left out.
pub fn read(m: Markup, waits: &mut Option<String>) -> Vec<(bool, String)> {
    match m {
        Markup::Typed { name, line } => {
            let mut out = vec![(true, line)];
            match at_once(&name) { Some(w) => { out.push((false, w.to_string())); *waits = None; } None => *waits = Some(name) }
            out
        }
        Markup::Printed(p) => waits.take().and_then(|n| then(&n, &p)).map(|w| vec![(false, w)]).unwrap_or_default(),
        Markup::Note => vec![],
    }
}
