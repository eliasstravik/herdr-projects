//! The board's keys, each one defined once: the ? menu, the hint line, the words that name a key and docs/keys.md are
//! all made from this list. To add or change a key, change its entry here and run `herdr-projects docs`.
//!
//! One rule sorts them: ctrl does something; alt only shows or hides a part of the screen and never changes anything.
//! A chord that is not in this list does nothing on the board. What each key does is board/update.rs, which matches
//! every key in every mode: a key with no arm there does not compile.
//!
//! An entry: `keys` the chords that press it (the first is the one shown; to move an action to another chord, change
//! this and nothing else); `raw` characters compared as typed ("?", " "); `fixed` the text box owns this chord; `show`
//! how it is written when that is not keys[0]; `gap` what stands between it and the keys described with it; `group`
//! where it stands in the ? menu and docs/keys.md; `with` described in another entry's row; `doc` what it does, for
//! docs/keys.md; `help` a plain key's line in the ? menu; `hint` its usual words on the hint line; `draft` the board's
//! key with a draft in the text box too; `web` what its button is called on the web board. In doc, help and hint,
//! another key is named as {name}, never by its chord. The order is the order everywhere.
use std::cell::Cell;

/// The groups, in the order the ? menu and docs/keys.md show them.
pub const GROUPS: [(&str, &str); 3] = [("plain", "plain keys"), ("ctrl", "ctrl: does something"), ("alt", "alt: only shows or hides")];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum K { Up, Down, PageUp, PageDown, Enter, Newline, Talk, Delegate, Back, Menu, SayYes, SayNo, Yes, SendBack, NextAttempt, Stop, Archive, Pin, Settings, PutBack, Undo, Find, Free, Close, ChatMessage, ChatScrollUp, ChatScrollDown, Side, Panel, ArchiveView, ByRepo, AllDone, Scope }
pub struct Key {
    pub id: K, pub name: &'static str, pub keys: &'static [&'static str], pub raw: &'static [&'static str], pub show: Option<&'static str>, pub gap: Option<&'static str>,
    pub group: Option<&'static str>, pub with: Option<&'static str>, pub fixed: bool, pub draft: bool, pub web: Option<&'static str>, pub help: Option<&'static str>,
    pub hint: Option<&'static str>, pub doc: Option<&'static str>,
}
pub static KEYS: &[Key] = &[
    Key { id: K::Up, name: "up", keys: &["up"], raw: &[], show: Some("↑"), gap: None, group: Some("plain"), with: None, fixed: false, draft: false, web: None, help: Some("move between tasks and headings"), hint: None,
        doc: Some("move between tasks and their headings, and on to the automations under them") },
    Key { id: K::Down, name: "down", keys: &["down"], raw: &[], show: Some("↓"), gap: None, group: None, with: Some("up"), fixed: false, draft: false, web: None, help: None, hint: None,
        doc: None },
    Key { id: K::PageUp, name: "pageUp", keys: &["pageUp"], raw: &[], show: Some("pgup"), gap: None, group: Some("plain"), with: None, fixed: false, draft: false, web: None, help: Some("a page of tasks up or down (in the whole task: a page of its text)"), hint: None,
        doc: Some("a page up or down: ten tasks in the list, a page of text in the whole task") },
    Key { id: K::PageDown, name: "pageDown", keys: &["pageDown"], raw: &[], show: Some("pgdn"), gap: None, group: None, with: Some("pageUp"), fixed: false, draft: false, web: None, help: None, hint: None,
        doc: None },
    Key { id: K::Enter, name: "enter", keys: &["enter", "return"], raw: &[], show: None, gap: None, group: Some("plain"), with: None, fixed: false, draft: false, web: Some("Open agent"), help: Some("send what is typed; else open this task's agent, or fold a heading"), hint: Some("open this task's agent"),
        doc: Some("send what you typed; with nothing typed, open the task's agent: its real herdr pane. The board keeps running in its own tab and is there as you left it; your key goes back to it (as a popup it closes, and your key opens it again where it was). On an automation's row: open the automation, whole ([automations.md](automations.md)). On a heading (a machine, a repo, a status group): fold it to its one line, which says how many tasks it holds, and unfold it; its caret says which it is, `▾` open, `▸` folded ([list-headings.md](list-headings.md))") },
    Key { id: K::Newline, name: "newline", keys: &["shift+enter"], raw: &[], show: None, gap: None, group: None, with: None, fixed: true, draft: false, web: None, help: None, hint: Some("new line"),
        doc: Some("a new line in what you are typing. Like the cursor keys and backspace, it is the text box's own editing key, not a key of the board") },
    Key { id: K::Talk, name: "talk", keys: &[], raw: &[" "], show: Some("space"), gap: None, group: Some("plain"), with: None, fixed: false, draft: false, web: Some("Talk"), help: Some("flip who you talk to: the Coordinator, or this task's agent"), hint: Some("talk to this task's agent"),
        doc: Some("flip who you talk to: the selected task's agent, and back to the Coordinator. The chat says who. Nothing else changes it: while it is a task's agent, moving with {up} {down} makes it the agent of the task you moved to. On a task with no agent, your message puts one on it. On an automation's row: turn the automation on or off") },
    Key { id: K::Delegate, name: "delegate", keys: &["tab"], raw: &[], show: None, gap: None, group: Some("plain"), with: None, fixed: false, draft: false, web: Some("Put an agent on this task"), help: Some("put an agent on this task"), hint: Some("put an agent on this task"),
        doc: Some("put an agent on the task. With text typed for the Coordinator: send it, and every task it adds from that message gets an agent at once, with nothing more to confirm (a message that adds no task delegates nothing). On an automation's row: fire the automation now") },
    Key { id: K::Back, name: "back", keys: &["escape"], raw: &[], show: Some("esc"), gap: None, group: Some("plain"), with: None, fixed: false, draft: false, web: Some("Back"), help: Some("back, one step at a time (as a popup, the board closes on the last one)"), hint: Some("back"),
        doc: Some("back, one step at a time, and nothing else: find is left, a draft is put away, then the whole task, the filter and the archive close, then the chat's latest message is hidden, as {chatMessage} hides it. With none of those left it shows the same hint as {close}; the board as a popup (`board.open`) closes on it. It never shows the chat and never changes who you talk to") },
    Key { id: K::Menu, name: "menu", keys: &[], raw: &["?"], show: Some("?"), gap: None, group: Some("plain"), with: None, fixed: false, draft: false, web: Some("More"), help: Some("this menu"), hint: Some("all keys"),
        doc: Some("the menu: every key in these three groups, each line with its key. A ctrl or alt line can be picked with {up} {down} and {enter}, which does what its key does. It opens with no line picked. Under the keys, what a row shows: each mark with its word, the numbers on the right, and what the colour of a review tag and of a pull request say. Every line is whole (a long one wraps); where the window is too short for all of it the last line says how many lines are below, and {down} on the last key scrolls on down, {up} on the first scrolls up") },
    Key { id: K::SayYes, name: "sayYes", keys: &["y", "shift+y"], raw: &["y", "Y"], show: Some("y"), gap: Some("  "), group: Some("plain"), with: None, fixed: false, draft: false, web: Some("Yes"), help: Some("answer a question: {sayYes} yes, {sayNo} or {back} no ({enter} never confirms)"), hint: None,
        doc: Some("answer a question the board asks: {sayYes} yes, {sayNo} or {back} no. Only while a question is up; {enter} never confirms") },
    Key { id: K::SayNo, name: "sayNo", keys: &["n", "shift+n"], raw: &["n", "N"], show: Some("n"), gap: None, group: None, with: Some("sayYes"), fixed: false, draft: false, web: Some("No"), help: None, hint: None,
        doc: None },
    Key { id: K::Yes, name: "yes", keys: &["ctrl+l"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Yes"), help: None, hint: Some("retry the failed step"),
        doc: Some("yes, to whatever the task waits for: approve the merge (it asks first), keep the selected one of several attempts (it asks first), say yes to the agent, retry the step that went wrong as it stands, or take work that stopped without a summary, or at a limit, as it is; on a task whose agent is at rest while its row still says \"Finishing\", finish it with what its agent said last") },
    Key { id: K::SendBack, name: "sendBack", keys: &["ctrl+r"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Send back"), help: None, hint: Some("tell this task's agent to fix it"),
        doc: Some("send it back: whatever went wrong (a failed check, a conflict, an error, a review with findings) goes to the task's agent to fix; a stopped agent is told to continue") },
    Key { id: K::NextAttempt, name: "nextAttempt", keys: &["ctrl+n"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Next attempt"), help: None, hint: Some("the next attempt"),
        doc: Some("on a task done several times at once ([attempts.md](attempts.md)): move to its next attempt, in the task panel. {yes} keeps the selected one, {enter} goes into its tab") },
    Key { id: K::Stop, name: "stop", keys: &["ctrl+x"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Stop"), help: None, hint: Some("stop this task's agent"),
        doc: Some("stop the agent, when one is running (working, starting, checking, under review, or waiting for your answer); the selection stays where you were, on the next row when the stopped task moves to another group. On a task with no agent running (idle, never started, stopped, finished, archived) it asks to remove the task instead: {sayYes} removes it, its worktree too. A task you marked as your own work in progress has no agent to stop: there the key puts it back in to do. A stop never goes on to that question by itself: removing takes a second {stop}, and a {sayYes}. On an automation's row: remove the automation (it asks first; the tasks it made stay)") },
    Key { id: K::Archive, name: "archive", keys: &["ctrl+a"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Archive"), help: None, hint: Some("restore this task"),
        doc: Some("archive the selected task, or restore it from the archive") },
    Key { id: K::Pin, name: "pin", keys: &["ctrl+p"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Pin"), help: None, hint: None,
        doc: Some("pin the selected task to the top, or unpin it") },
    Key { id: K::Settings, name: "settings", keys: &["ctrl+e"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Settings"), help: None, hint: None,
        doc: Some("the selected task's settings, full screen: every work setting by its name and its value. {up} {down} move to a row; {enter} changes it (a switch or a choice goes to its next value, anything else is typed and saved with {enter}); rows set for this task only are marked `*` (where any other value comes from is `herdr-projects show <id>`'s). {putBack} puts the selected row back to what it would be without this task's own answer. {back} or {settings} again goes back ([settings.md](settings.md)). On an automation's row: the automation's own settings, the same page") },
    Key { id: K::PutBack, name: "putBack", keys: &["ctrl+u"], raw: &[], show: None, gap: None, group: None, with: Some("settings"), fixed: false, draft: false, web: None, help: None, hint: None,
        doc: None },
    Key { id: K::Undo, name: "undo", keys: &["ctrl+z"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Undo"), help: None, hint: None,
        doc: Some("undo: bring back a draft that was put away, or take the last archiving back") },
    Key { id: K::Find, name: "find", keys: &["ctrl+f"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: Some("Find"), help: None, hint: None,
        doc: Some("find: type, and the list shows only the tasks that match, archived ones too. {enter} keeps the filter, {back} leaves find; {back} on the list clears a filter that is kept") },
    Key { id: K::Free, name: "free", keys: &["ctrl+k"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: None, help: None, hint: None,
        doc: Some("only while a remote view holds the board's terminal at another size than herdr shows it (the line at the top says so): end that view, so the board has its own size again ([board-size.md](board-size.md))") },
    Key { id: K::Close, name: "close", keys: &["ctrl+c"], raw: &[], show: None, gap: None, group: Some("ctrl"), with: None, fixed: false, draft: false, web: None, help: None, hint: None,
        doc: Some("twice: close the board, with text typed too: the draft is kept and is back when the board reopens") },
    Key { id: K::ChatMessage, name: "chatMessage", keys: &["alt+c"], raw: &[], show: None, gap: None, group: Some("alt"), with: None, fixed: false, draft: false, web: None, help: None, hint: None,
        doc: Some("the chat's latest message, the Coordinator's or a task agent's. The one key for it, both ways") },
    Key { id: K::ChatScrollUp, name: "chatScrollUp", keys: &["alt+k"], raw: &[], show: None, gap: None, group: Some("alt"), with: None, fixed: false, draft: true, web: None, help: None, hint: None,
        doc: Some("more of a chat message that is longer than its box: {chatScrollUp} scrolls it up, {chatScrollDown} down, half a box at a time, also while you type. Up opens the box to the conversation before the newest message, each press further up; it stays open, whatever comes meanwhile, until {chatScrollDown} at its end goes back to the newest message ({chatMessage} hides and shows the whole chat; talking to another starts it at the newest too). The box says where there is more, and names the key (`alt+k more ↑`, `alt+j more ↓`). The mouse wheel does the same with the pointer over the chat ([mouse.md](mouse.md))") },
    Key { id: K::ChatScrollDown, name: "chatScrollDown", keys: &["alt+j"], raw: &[], show: None, gap: None, group: Some("alt"), with: Some("chatScrollUp"), fixed: false, draft: true, web: None, help: None, hint: None,
        doc: None },
    Key { id: K::Side, name: "side", keys: &["alt+p"], raw: &[], show: None, gap: None, group: Some("alt"), with: None, fixed: false, draft: false, web: None, help: None, hint: None,
        doc: Some("the side panel: the selected task or automation, beside the list") },
    Key { id: K::Panel, name: "panel", keys: &["alt+t"], raw: &[], show: None, gap: None, group: Some("alt"), with: None, fixed: false, draft: false, web: Some("Open the task"), help: None, hint: None,
        doc: Some("the whole task (on an automation's row: the whole automation), full screen, and back to the list") },
    Key { id: K::ArchiveView, name: "archiveView", keys: &["alt+a"], raw: &[], show: None, gap: None, group: Some("alt"), with: None, fixed: false, draft: false, web: Some("Show the archive"), help: None, hint: None,
        doc: Some("the archive, and back to the board") },
    Key { id: K::ByRepo, name: "byRepo", keys: &["alt+s"], raw: &[], show: None, gap: None, group: Some("alt"), with: None, fixed: false, draft: false, web: Some("Group by repo"), help: None, hint: None,
        doc: Some("the list grouped by repo, each group under its repo's name (a row's tag never names a repo), or by status") },
    Key { id: K::AllDone, name: "allDone", keys: &["alt+d"], raw: &[], show: None, gap: None, group: Some("alt"), with: None, fixed: false, draft: false, web: Some("Show all done"), help: None, hint: None,
        doc: Some("every done task, or only the latest") },
    Key { id: K::Scope, name: "scope", keys: &["alt+m"], raw: &[], show: None, gap: None, group: Some("alt"), with: None, fixed: false, draft: false, web: Some("Other machines"), help: None, hint: None,
        doc: Some("other machines: this machine only, every machine, then each machine by itself") },
];
pub fn entry(id: K) -> &'static Key { KEYS.iter().find(|k| k.id == id).expect("every key has its entry") }
pub fn by_name(name: &str) -> Option<&'static Key> { KEYS.iter().find(|k| k.name == name) }

/// A key press as the board reads it, from a terminal or from the web board: the chord ("ctrl+l", "shift+enter",
/// "up") and the character typed, when one was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Press { pub chord: String, pub ch: Option<char> }
impl Press {
    pub fn chord(c: &str) -> Self { Press { chord: c.into(), ch: None } }
    pub fn char(c: char) -> Self { Press { chord: if c.is_uppercase() { format!("shift+{}", c.to_lowercase()) } else { c.to_string() }, ch: Some(c) } }
}
/// One spelling of a chord: "return" is enter, "esc" is escape, modifiers in one order.
pub fn norm(chord: &str) -> String {
    let low = chord.to_lowercase();
    let parts: Vec<&str> = low.split('+').collect();
    let (mods, key) = parts.split_at(parts.len().saturating_sub(1));
    let key = match key.first().copied().unwrap_or("") { "return" => "enter", "esc" => "escape", "pgup" => "pageup", "pgdn" => "pagedown", "space" => " ", k => k };
    let mut out: Vec<&str> = ["ctrl", "alt", "shift"].into_iter().filter(|m| mods.contains(m)).collect();
    out.push(key);
    out.join("+")
}
/// Is this press the key `id`?
pub fn is(p: &Press, id: K) -> bool {
    let k = entry(id);
    if let Some(c) = p.ch { if k.raw.iter().any(|r| r.chars().eq(std::iter::once(c))) { return true; } }
    let typed = norm(&p.chord);
    k.keys.iter().any(|c| norm(c) == typed)
}
/// The key that was pressed, among those `pick` lets through.
pub fn typed(p: &Press, pick: impl Fn(&Key) -> bool) -> Option<K> { KEYS.iter().find(|k| pick(k) && is(p, k.id)).map(|k| k.id) }

thread_local! { static BUTTONS: Cell<bool> = const { Cell::new(false) }; }
/// The web board has buttons, not keys: there a key is named as its button is ("“Undo” brings it back").
pub fn name_as_buttons(on: bool) { BUTTONS.with(|b| b.set(on)); }
fn own(k: &Key) -> String { if BUTTONS.with(|b| b.get()) { if let Some(w) = k.web { return format!("“{w}”"); } } k.show.unwrap_or(k.keys.first().copied().unwrap_or("")).to_string() }
/// The key called `id` as the board writes it: "ctrl+a".
pub fn name(id: K) -> String { own(entry(id)) }
/// With the keys described in the same row: "↑ ↓".
pub fn pair(id: K) -> String {
    let k = entry(id);
    std::iter::once(k).chain(KEYS.iter().filter(|x| x.with == Some(k.name))).map(own).collect::<Vec<_>>().join(k.gap.unwrap_or(" "))
}
/// The key with this name, as the board writes it. Words elsewhere name a key by its name: key_name("yes").
pub fn key_name(n: &str) -> String { by_name(n).map(own).unwrap_or_else(|| panic!("No key is called \"{n}\" (crates/flow/src/board/keys.rs)")) }
/// Words that name other keys as {name}, with each key as the board writes it.
pub fn say(text: &str) -> String {
    let re = crate::re!(r"\{(\w+)\}");
    re.replace_all(text, |c: &regex::Captures| by_name(&c[1]).map(own).unwrap_or_else(|| c[0].to_string())).into_owned()
}
/// A pair for the hint line: the key and its usual words, or the words this moment asks for.
pub fn hint(id: K, text: Option<&str>) -> (String, String) { (name(id), say(text.unwrap_or(entry(id).hint.unwrap_or("")))) }
/// The same key as a page writes it: `Ctrl+A`, `Esc`, `↑` `↓`.
pub fn written(shown: &str) -> String {
    shown.split_whitespace().map(|c| {
        let has_plus = c.contains('+');
        let words: Vec<String> = c.split('+').map(|w| {
            let caps = (w.len() >= 2 && w.chars().all(|ch| ch.is_ascii_lowercase())) || (has_plus && w.len() == 1 && w.chars().all(|ch| ch.is_ascii_lowercase()));
            if caps { let mut s = w.to_string(); s[..1].make_ascii_uppercase(); s } else { w.to_string() }
        }).collect();
        format!("`{}`", words.join("+"))
    }).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_presses() {
        assert_eq!(key_name("yes"), "ctrl+l");
        assert_eq!(pair(K::Up), "↑ ↓");
        assert_eq!(pair(K::SayYes), "y  n");
        assert!(is(&Press::char('?'), K::Menu));
        assert!(is(&Press::char('Y'), K::SayYes));
        assert!(is(&Press::chord("return"), K::Enter));
        assert_eq!(written("ctrl+a"), "`Ctrl+A`");
        assert_eq!(say("{back} says no"), "esc says no");
    }
}
