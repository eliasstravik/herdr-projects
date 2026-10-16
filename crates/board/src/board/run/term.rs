//! The terminal: taken full screen with the mouse and bracketed paste, and given back exactly as it was on every way
//! out (closing, a panic, a signal), so nothing is left in your shell. Before that the terminal is asked for its own
//! text and background colours, which the greys are measured against.
use crossterm::{cursor, event, execute, terminal};
use std::io::{Read, Write};

/// A board that loads its new version keeps its screen: the new one draws over the old frame instead of clearing.
pub const KEPT: &str = "HERDR_PROJECTS_KEPT_SCREEN";
pub fn enter() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    if std::env::var(KEPT).as_deref() != Ok("1") { execute!(out, terminal::EnterAlternateScreen)?; }
    unsafe { std::env::remove_var(KEPT); }
    execute!(out, cursor::Hide, event::EnableMouseCapture, event::EnableBracketedPaste)?;
    // The kitty keyboard protocol, where the terminal has it: shift+enter and alt+letter arrive as themselves.
    let _ = execute!(out, event::PushKeyboardEnhancementFlags(event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES));
    Ok(())
}
/// Everything given back. `keep`: the full-screen view stays (a reload draws over it).
pub fn leave(keep: bool) {
    let mut out = std::io::stdout();
    let _ = execute!(out, event::PopKeyboardEnhancementFlags);
    let _ = execute!(out, event::DisableBracketedPaste, event::DisableMouseCapture, cursor::Show);
    if !keep { let _ = execute!(out, terminal::LeaveAlternateScreen); }
    let _ = terminal::disable_raw_mode();
    let _ = out.flush();
}
/// A panic leaves the terminal as it found it before saying what happened.
pub fn guard_panics() {
    let old = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| { if crate::board::view::guard::quiet() { return; } leave(false); old(info); }));
}
/// Ask the terminal for its text colour and its background, and whether it is light. Never waits long: the question
/// for the terminal's kind goes last, and every terminal answers that one.
/// The terminal is asked for its colours; the answers are read with `colours_told`, so the board loads meanwhile.
/// Asked: false when it is no terminal (nothing is asked then).
pub fn ask_colours() -> bool {
    use std::io::IsTerminal;
    // Windows: the console cannot be waited on for an answer the way a Unix terminal can, so nothing is asked.
    if cfg!(windows) { return false; }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() || terminal::enable_raw_mode().is_err() { return false; }
    let mut out = std::io::stdout();
    let _ = out.write_all(b"\x1b]10;?\x1b\\\x1b]11;?\x1b\\\x1b[c");
    let _ = out.flush();
    true
}
/// What the terminal said of its colours, waited for at most `ms` from now; COLORFGBG where it said nothing.
pub fn colours_told(asked: bool, ms: u64) -> crate::board::theme::Seen {
    let fgbg = std::env::var("COLORFGBG").ok().and_then(|v| v.rsplit(';').next().and_then(|x| x.parse::<u8>().ok())).map(|n| n == 7 || n == 15);
    let mut seen = crate::board::theme::Seen { light: fgbg, fg: None, bg: None };
    if !asked { return seen; }
    let mut buf = Vec::new();
    let until = std::time::Instant::now() + std::time::Duration::from_millis(ms);
    let done = |b: &[u8]| crate::board::theme::answers::read_answers(&String::from_utf8_lossy(b)).1;
    while std::time::Instant::now() < until && !done(&buf) {
        let left = until.saturating_duration_since(std::time::Instant::now()).as_millis() as i32;
        if !input_within(left.max(1)) { break; }
        let mut chunk = [0u8; 256];
        match std::io::stdin().read(&mut chunk) { Ok(n) if n > 0 => buf.extend_from_slice(&chunk[..n]), _ => break }
    }
    let _ = terminal::disable_raw_mode();
    let (told, _) = crate::board::theme::answers::read_answers(&String::from_utf8_lossy(&buf));
    seen.fg = told.fg;
    seen.bg = told.bg;
    if told.light.is_some() { seen.light = told.light; }
    seen
}
/// Whether stdin has something to read within `ms`.
#[cfg(unix)]
fn input_within(ms: i32) -> bool {
    let mut p = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
    unsafe { libc::poll(&mut p, 1, ms) > 0 }
}
#[cfg(windows)]
fn input_within(_: i32) -> bool { false }
