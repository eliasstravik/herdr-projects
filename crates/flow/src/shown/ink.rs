//! Words with a named ink: what the board's parts say, each piece with the name of the colour it is drawn in. The
//! names are the board's own (herdr's theme tokens and a few of the board's), never a colour: the terminal drawer
//! (board/draw/theme.rs) and the web page each turn a name into their own colour.
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Ink {
    /// The terminal's own text, nothing painted.
    #[default]
    Plain,
    /// herdr's plain text colour.
    Text,
    /// The brighter grey: words that have to be read (herdr's subtext0).
    Soft,
    /// The fainter grey: what may be missed (times, counts, tags; herdr's overlay0, held readable).
    Muted,
    Red,
    Green,
    Yellow,
    Teal,
    Peach,
    Mauve,
    Accent,
    /// The reviewer is at work: a blue told apart from everything beside it.
    Review,
    /// The board runs the check itself: a plain light grey.
    Check,
    /// It waits for a free slot: a pink no other state has.
    Queue,
    /// Nobody has started it: the dim grey of "to do".
    Todo,
    /// The automations' own purple.
    Auto,
    /// herdr's thin rules and frames (surface1), its dimmest separators (surface_dim).
    Rule,
    Dim,
    /// Backgrounds: herdr's sidebar, and the selected row (selection_bg, or active_row_bg where the theme has none).
    Sidebar,
    Selection,
}
impl Ink {
    /// The name the web page and the snapshots use.
    pub fn name(self) -> &'static str {
        match self { Ink::Plain => "plain", Ink::Text => "text", Ink::Soft => "subtext0", Ink::Muted => "overlay0", Ink::Red => "red", Ink::Green => "green",
            Ink::Yellow => "yellow", Ink::Teal => "teal", Ink::Peach => "peach", Ink::Mauve => "mauve", Ink::Accent => "accent", Ink::Review => "review",
            Ink::Check => "check", Ink::Queue => "queue", Ink::Todo => "todo", Ink::Auto => "auto", Ink::Rule => "surface1", Ink::Dim => "surface_dim",
            Ink::Sidebar => "sidebar_bg", Ink::Selection => "selection" }
    }
}
fn no(b: &bool) -> bool { !*b }
/// One piece of words in one ink. `quiet`: the ink made quieter and still seen (a finished mark); `inv`: drawn in
/// reverse (the cursor, a selection where no background can be painted); `bg`: a background ink; `link`: the words
/// open this address where the terminal or the page can.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct Piece {
    pub t: String, pub c: Ink,
    #[serde(skip_serializing_if = "no")] pub bold: bool, #[serde(skip_serializing_if = "no")] pub dim: bool,
    #[serde(skip_serializing_if = "no")] pub quiet: bool, #[serde(skip_serializing_if = "no")] pub under: bool, #[serde(skip_serializing_if = "no")] pub inv: bool,
    #[serde(skip_serializing_if = "Option::is_none")] pub bg: Option<Ink>, #[serde(skip_serializing_if = "Option::is_none")] pub link: Option<String>,
    /// What the line this piece starts is, where a page lays it out by itself (the task panel's parts on the web board):
    /// "head", "fact:<label>", "more", "text". Never drawn.
    #[serde(skip_serializing_if = "Option::is_none")] pub role: Option<String>,
}
/// Words in inks, in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct Inked(pub Vec<Piece>);
impl Inked {
    pub fn new() -> Self { Inked(vec![]) }
    pub fn of(t: impl Into<String>, c: Ink) -> Self { let t = t.into(); if t.is_empty() { Inked(vec![]) } else { Inked(vec![Piece { t, c, ..Default::default() }]) } }
    pub fn plain(t: impl Into<String>) -> Self { Self::of(t, Ink::Plain) }
    pub fn push(&mut self, t: impl Into<String>, c: Ink) -> &mut Self { let t = t.into(); if !t.is_empty() { self.0.push(Piece { t, c, ..Default::default() }); } self }
    pub fn bold(mut self) -> Self { for p in &mut self.0 { p.bold = true; } self }
    pub fn dim(mut self) -> Self { for p in &mut self.0 { p.dim = true; } self }
    pub fn quiet(mut self) -> Self { for p in &mut self.0 { p.quiet = true; } self }
    pub fn under(mut self) -> Self { for p in &mut self.0 { p.under = true; } self }
    pub fn inv(mut self) -> Self { for p in &mut self.0 { p.inv = true; } self }
    pub fn link(mut self, url: &str) -> Self { for p in &mut self.0 { p.link = Some(url.to_string()); } self }
    /// A background under every piece that has none of its own.
    pub fn on(mut self, bg: Ink) -> Self { for p in &mut self.0 { if p.bg.is_none() { p.bg = Some(bg); } } self }
    /// Pieces in their inks, joined: `Inked::join([a, b])`.
    /// The line marked as what it is, for a page (see Piece::role).
    pub fn role(mut self, role: impl Into<String>) -> Self { if self.0.is_empty() { self.0.push(Piece::default()); } self.0[0].role = Some(role.into()); self }
    pub fn role_of(&self) -> Option<&str> { self.0.first().and_then(|p| p.role.as_deref()) }
    pub fn join(parts: impl IntoIterator<Item = Inked>) -> Self { let mut out = Inked::new(); for p in parts { out.0.extend(p.0); } out }
    pub fn spaces(n: usize) -> Self { Inked::plain(" ".repeat(n)) }
    /// The same words, `n` cells wide: cut with `…`, or filled with spaces.
    pub fn fit(&self, n: usize) -> Inked { let c = self.cut(n); let w = c.width(); if w < n { c.add(Inked::spaces(n - w)) } else { c } }
    /// Filled with spaces up to `n` cells, never cut.
    pub fn pad(&self, n: usize) -> Inked { let w = self.width(); if w < n { self.clone().add(Inked::spaces(n - w)) } else { self.clone() } }
    /// The pieces from cell `from` on (a piece cut in the middle of a wide character loses it).
    pub fn skip(&self, from: usize) -> Inked {
        let mut out = Inked::new(); let mut at = 0;
        for p in &self.0 {
            let w = crate::text::width(&p.t);
            if at >= from { out.0.push(p.clone()); } else if at + w > from { let t = crate::text::tail(&p.t, at + w - from).to_string(); if !t.is_empty() { out.0.push(Piece { t, ..p.clone() }); } }
            at += w;
        }
        out
    }
    /// Without the spaces at its end.
    pub fn trim_end(mut self) -> Inked {
        while let Some(p) = self.0.last_mut() { let t = p.t.trim_end().to_string(); if t.is_empty() { self.0.pop(); } else { p.t = t; break; } }
        self
    }
    /// Wrapped to lines of at most `max` cells, as the board wraps every text: at spaces, a word longer than a line
    /// broken where it must be, each line without the spaces at its end. Line breaks are kept; the inks go on.
    pub fn wrap(&self, max: usize) -> Vec<Inked> { super::wrap::wrap(self, max) }
    /// The words split at their line breaks, each line in its inks.
    pub fn lines(&self) -> Vec<Inked> {
        let mut out = vec![Inked::new()];
        for p in &self.0 {
            for (n, part) in p.t.split('\n').enumerate() {
                if n > 0 { out.push(Inked::new()); }
                if !part.is_empty() { out.last_mut().unwrap().0.push(Piece { t: part.to_string(), ..p.clone() }); }
            }
        }
        out
    }
    pub fn add(mut self, other: Inked) -> Self { self.0.extend(other.0); self }
    pub fn text(&self) -> String { self.0.iter().map(|p| p.t.as_str()).collect() }
    pub fn is_empty(&self) -> bool { self.0.iter().all(|p| p.t.is_empty()) }
    pub fn width(&self) -> usize { crate::text::width(&self.text()) }
    /// The same pieces, every ink replaced.
    pub fn recolor(mut self, c: Ink) -> Self { for p in &mut self.0 { p.c = c; } self }
    /// Cut to `max` cells with `…` (in the ink of the last piece kept).
    pub fn cut(&self, max: usize) -> Inked {
        if self.width() <= max { return self.clone(); }
        if max == 0 { return Inked::new(); }
        let mut out = Inked::new();
        let mut room = max - 1;
        let mut last = Ink::Plain;
        for p in &self.0 {
            let w = crate::text::width(&p.t);
            if w <= room { out.0.push(p.clone()); room -= w; last = p.c; continue; }
            let h = crate::text::head(&p.t, room).to_string();
            if !h.is_empty() { out.0.push(Piece { t: h, ..p.clone() }); }
            last = p.c;
            break;
        }
        out.push("…", last);
        out
    }
}
impl From<&str> for Inked { fn from(s: &str) -> Self { Inked::plain(s) } }
impl From<String> for Inked { fn from(s: String) -> Self { Inked::plain(s) } }
