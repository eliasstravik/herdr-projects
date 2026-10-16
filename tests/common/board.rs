//! A board opened on a test world, as a terminal would show it: keys pressed by their chord ("alt+t", "esc", "ctrl+l")
//! or typed as text, what they started run at once (the jobs a terminal board runs on threads), and the screen read as
//! plain text. Also the web board's calls on the same world.
use serde_json::Value;
use sw::board::data::{load, Wants};
use sw::board::job;
use sw::board::keys::Press;
use sw::board::state::{Ctx, State};
use sw::board::theme::Theme;
use sw::board::update::{list::relist, update, Effect, Event};

/// The chat as the screen shows it: the line that says who, what is inside its box (without the frame), the hints.
pub struct Chat { pub framed: bool, pub who: String, pub boxed: Vec<String>, pub hints: String, pub lines: Vec<String> }
pub struct Draft { pub rows: Vec<String>, pub above: usize, pub below: usize, pub cursor: (usize, usize), pub lines: Vec<String>, pub framed: bool }
pub struct Board { pub s: State, pub rows: usize, pub cols: usize, pub closed: bool, pub theme: Theme }
pub fn open(cwd: &std::path::Path, rows: usize) -> Board { open_with(cwd, rows, false) }
/// `popup`: the board in herdr's popup, which closes on the last esc and when an agent is opened.
pub fn open_with(cwd: &std::path::Path, rows: usize, popup: bool) -> Board {
    let p = sw::board::run::prefs::read(popup);
    let ctx = Ctx { cwd: cwd.display().to_string(), can_reload: true, leave_ms: 0, settle_ms: 0, popup, in_herdr: sw::herdr::cli::in_herdr(), ..Default::default() };
    let mut s = State::new(ctx, Default::default());
    sw::board::run::prefs::apply(&mut s, &p);
    s.side = true;
    let mut b = Board { s, rows, cols: 120, closed: false, theme: Theme::default_dark(true) };
    b.load();
    relist(&mut b.s);
    b.load(); // what the selected task has a closer look for
    b
}
impl Board {
    pub fn load(&mut self) {
        let d = load(&Wants { ids: vec![self.s.sel.clone(), self.s.reply_id.clone()], cwd: self.s.ctx.cwd.clone(), own_pane: String::new(), web: false, coord: sw::board::update::coords::wanted(&self.s) });
        let fx = update(&mut self.s, Event::Loaded(Box::new(d)));
        self.effects(fx);
    }
    fn effects(&mut self, fx: Vec<Effect>) {
        let mut again = false;
        for e in fx {
            match e {
                Effect::Run(j) => { let d = job::run(j); let more = update(&mut self.s, Event::Done(d)); self.effects(more); }
                Effect::Load => again = true,
                Effect::Close => self.closed = true,
                Effect::Save => sw::board::run::prefs::save(&self.s),
                _ => {}
            }
        }
        if again { self.load(); }
    }
    /// One key pressed, and what it started left for `run`: the board as it is the moment the key is pressed, before any
    /// of its work is done.
    pub fn hold(&mut self, key: &str) -> Vec<Effect> { update(&mut self.s, Event::Key(press_of(key))) }
    /// What a held key started, run now (its job, and the read after it).
    pub fn run(&mut self, fx: Vec<Effect>) { self.effects(fx); }
    /// Keys by their chord ("alt+t", "esc", "enter", "space", "ctrl+l"); anything else is one character typed.
    pub fn press(&mut self, keys: &[&str]) {
        for k in keys {
            let p = press_of(k);
            let was = self.s.sel.clone();
            let fx = update(&mut self.s, Event::Key(p));
            self.effects(fx);
            if self.s.sel != was { self.load(); } // the loader looks closer at the new selection
        }
    }
    pub fn typed(&mut self, text: &str) { for c in text.chars() { let fx = update(&mut self.s, Event::Key(Press::char(c))); self.effects(fx); } }
    pub fn lines(&mut self) -> Vec<String> {
        let sc = sw::board::view::screen(&mut self.s, &self.theme, self.cols, self.rows);
        sc.lines.iter().map(|l| l.text()).collect()
    }
    pub fn text(&mut self) -> String { self.lines().join("\n") }
    /// The screen's lines in their named inks, as the drawer gets them.
    pub fn inked(&mut self, cols: usize) -> Vec<sw::shown::ink::Inked> { sw::board::view::screen(&mut self.s, &self.theme, cols, self.rows).lines }
    pub fn wheel(&mut self, y: u16, by: i64) { let fx = update(&mut self.s, Event::Wheel { x: 5, y, by }); self.effects(fx); }
    pub fn click(&mut self, y: u16) { let fx = update(&mut self.s, Event::Click { x: 5, y }); self.effects(fx); }
    pub fn chat(&mut self, cols: usize) -> Chat {
        let lines = self.screen(cols);
        let top = lines.iter().rposition(|l| l.starts_with("╭─") && l.ends_with('╮'));
        let end = lines.iter().rposition(|l| l.starts_with("╰─") && l.ends_with('╯'));
        match (top, end) {
            (Some(top), Some(end)) => {
                for l in &lines[top..=end] { assert_eq!(sw::text::width(l), cols, "the box is as wide as the window: {l:?}"); }
                let inner = |l: &String| { let c: Vec<char> = l.chars().collect(); c[2..c.len() - 2].iter().collect::<String>().trim_end().to_string() };
                Chat { framed: true, who: lines[top - 1].clone(), boxed: lines[top + 1..end].iter().map(inner).collect(), hints: lines.get(end + 1).cloned().unwrap_or_default().trim().to_string(), lines }
            }
            _ => { let at = lines.iter().rposition(|l| l.starts_with('❯')).unwrap_or(0); Chat { framed: false, who: lines[at.saturating_sub(2)].clone(), boxed: vec![lines[at].clone()], hints: lines.last().cloned().unwrap_or_default().trim().to_string(), lines } }
        }
    }
    pub fn paste(&mut self, text: &str) { let fx = update(&mut self.s, Event::Paste(text.into())); self.effects(fx); }
    /// The text box as the screen shows it at this width: its rows without the prompt, how many rows it hides above and
    /// below, where the cursor is (row in the box, cell in the row), all the lines, and whether it sits in the chat's frame.
    pub fn draft(&mut self, cols: usize) -> Draft {
        let was = self.cols; self.cols = cols;
        let sc = sw::board::view::screen(&mut self.s, &self.theme, cols, self.rows);
        self.cols = was;
        let lines: Vec<String> = sc.lines.iter().map(|l| l.text().trim_end().to_string()).collect();
        let framed = lines.iter().any(|l| l.starts_with("│ ❯ "));
        let inner: Vec<String> = lines.iter().map(|l| if framed && l.starts_with("│ ") && l.ends_with(" │") { let c: Vec<char> = l.chars().collect(); c[2..c.len() - 2].iter().collect::<String>().trim_end().to_string() } else { l.clone() }).collect();
        let first = inner.iter().position(|l| l.starts_with("❯ ") || l == "❯").unwrap_or(0);
        let last = inner.iter().enumerate().position(|(n, l)| n > first && (l.starts_with('─') || l.starts_with('╰'))).unwrap_or(inner.len());
        let hidden = |l: Option<&String>| l.and_then(|l| regex::Regex::new(r"(\d+) more").unwrap().captures(l).map(|c| c[1].parse::<usize>().unwrap_or(0))).unwrap_or(0);
        let rows: Vec<String> = inner[first..last].iter().map(|l| l.chars().skip(2).collect::<String>().trim_end().to_string()).collect();
        let cursor = sc.cursor.map(|(x, y)| (y as usize - first, x as usize - 2 - if framed { 2 } else { 0 })).unwrap_or((0, 0));
        Draft { above: hidden(if first > 0 { inner.get(first - 1) } else { None }), below: hidden(inner.get(last)), rows, cursor, lines, framed }
    }
    /// The screen at another width, each line without the spaces at its end.
    pub fn screen(&mut self, cols: usize) -> Vec<String> { let was = self.cols; self.cols = cols; let l = self.lines(); self.cols = was; l.into_iter().map(|x| x.trim_end().to_string()).collect() }
}
fn press_of(k: &str) -> Press { match k { "space" => Press::char(' '), "esc" => Press::chord("escape"), k if k.chars().count() == 1 => Press::char(k.chars().next().unwrap()), k => Press::chord(k) } }
/// One call of the web board, as the page makes it (for a page that may drive).
pub fn web(w: &super::World, call: &str, args: Value) -> Value { web_as(w, "test", call, args).unwrap_or_else(|e| panic!("web {call}: {e}")) }
/// A call of one browser's board (`client`), as a page that may drive unless `args` says its `role`.
pub fn web_as(w: &super::World, client: &str, call: &str, args: Value) -> Result<Value, String> {
    let view = sw::store::files::data_dir().join("view.json");
    if !view.exists() { let _ = std::fs::write(&view, serde_json::json!({ "cwd": w.root_str() }).to_string()); }
    let mut a = args; if a.get("role").is_none() { a["role"] = serde_json::json!("drive"); }
    sw::webui::web::call(client, call, a, "", None)
}
