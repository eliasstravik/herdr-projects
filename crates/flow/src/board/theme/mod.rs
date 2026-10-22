//! The board's colours and status marks are herdr's own: the palettes are herdr's code itself (herdr/palette.rs), and
//! the board reads the same `[theme]` and `[ui]` settings herdr reads from its config.toml and resolves them the way
//! herdr does, so with no theme set it is herdr's default, Catppuccin, exactly as herdr shows it. herdr does not hand
//! its theme to plugins, and a herdr opened from another machine paints with that machine's config: the board's own
//! `board.theme` setting names a theme by hand then. The board's own colours are only for what herdr has no state for:
//! "review" (a blue), "todo" (a dim grey), "check" (herdr's subtext0), "queue" (a pink) and "auto" (a
//! purple) (picks.rs). The greys are herdr's, made readable (greys.rs). With NO_COLOR set nothing is painted: marks, words
//! and reverse video still say everything.
pub mod answers;
pub mod greys;
pub mod marks;
pub mod page;
pub mod picks;

use crate::herdr::palette::{canonical_theme_name, parse_color, Palette};
use crate::shown::ink::Ink;
use ratatui::style::Color;

/// A colour as the board measures it: the terminal's own (none set), one of its 16, or a real one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum C { Own, Ansi(u8), Rgb(u32) }
impl C {
    pub fn of(c: Color) -> C {
        match c {
            Color::Reset => C::Own, Color::Rgb(r, g, b) => C::Rgb((r as u32) << 16 | (g as u32) << 8 | b as u32),
            Color::Black => C::Ansi(0), Color::Red => C::Ansi(1), Color::Green => C::Ansi(2), Color::Yellow => C::Ansi(3), Color::Blue => C::Ansi(4), Color::Magenta => C::Ansi(5), Color::Cyan => C::Ansi(6), Color::Gray => C::Ansi(7),
            Color::DarkGray => C::Ansi(8), Color::LightRed => C::Ansi(9), Color::LightGreen => C::Ansi(10), Color::LightYellow => C::Ansi(11), Color::LightBlue => C::Ansi(12), Color::LightMagenta => C::Ansi(13), Color::LightCyan => C::Ansi(14), Color::White => C::Ansi(15),
            Color::Indexed(n) if n < 16 => C::Ansi(n), _ => C::Own,
        }
    }
    pub fn color(self) -> Color { match self { C::Own => Color::Reset, C::Ansi(n) => Color::Indexed(n), C::Rgb(x) => Color::Rgb((x >> 16) as u8, (x >> 8) as u8, x as u8) } }
    /// Only a real colour can be measured.
    pub fn real(self) -> Option<u32> { match self { C::Rgb(x) => Some(x), _ => None } }
    /// As near as can be known to what is drawn: one of the 16 as xterm draws it.
    pub fn drawn(self) -> Option<u32> { match self { C::Rgb(x) => Some(x), C::Ansi(n) => Some(greys::XTERM[n as usize & 15]), C::Own => None } }
}
/// herdr's tokens and the board's own colours, resolved.
#[derive(Clone, Debug)]
pub struct Pal {
    pub accent: C, pub panel_bg: C, pub sidebar_bg: C, pub active_row_bg: C, pub selection_bg: C, pub surface0: C, pub surface1: C, pub surface_dim: C,
    pub overlay0: C, pub overlay1: C, pub text: C, pub subtext0: C, pub mauve: C, pub green: C, pub yellow: C, pub red: C, pub blue: C, pub teal: C, pub peach: C,
    pub review: C, pub check: C, pub todo: C, pub auto: C, pub queue: C,
}
pub const TOKENS: [&str; 19] = ["accent", "panel_bg", "sidebar_bg", "active_row_bg", "selection_bg", "surface0", "surface1", "surface_dim", "overlay0", "overlay1", "text", "subtext0", "mauve", "green", "yellow", "red", "blue", "teal", "peach"];
impl Pal {
    pub fn from(p: &Palette) -> Pal {
        let c = C::of;
        Pal { accent: c(p.accent), panel_bg: c(p.panel_bg), sidebar_bg: c(p.sidebar_bg), active_row_bg: c(p.active_row_bg), selection_bg: c(p.selection_bg), surface0: c(p.surface0), surface1: c(p.surface1), surface_dim: c(p.surface_dim),
            overlay0: c(p.overlay0), overlay1: c(p.overlay1), text: c(p.text), subtext0: c(p.subtext0), mauve: c(p.mauve), green: c(p.green), yellow: c(p.yellow), red: c(p.red), blue: c(p.blue), teal: c(p.teal), peach: c(p.peach),
            review: C::Own, check: C::Own, todo: C::Own, auto: C::Own, queue: C::Own }
    }
    pub fn get(&self, k: &str) -> C {
        match k { "accent" => self.accent, "panel_bg" => self.panel_bg, "sidebar_bg" => self.sidebar_bg, "active_row_bg" => self.active_row_bg, "selection_bg" => self.selection_bg, "surface0" => self.surface0, "surface1" => self.surface1,
            "surface_dim" => self.surface_dim, "overlay0" => self.overlay0, "overlay1" => self.overlay1, "text" => self.text, "subtext0" => self.subtext0, "mauve" => self.mauve, "green" => self.green, "yellow" => self.yellow,
            "red" => self.red, "blue" => self.blue, "teal" => self.teal, "peach" => self.peach, "review" => self.review, "check" => self.check, "todo" => self.todo, "auto" => self.auto, "queue" => self.queue, _ => C::Own }
    }
    pub fn set(&mut self, k: &str, v: C) {
        let slot = match k { "accent" => &mut self.accent, "panel_bg" => &mut self.panel_bg, "sidebar_bg" => &mut self.sidebar_bg, "active_row_bg" => &mut self.active_row_bg, "selection_bg" => &mut self.selection_bg, "surface0" => &mut self.surface0,
            "surface1" => &mut self.surface1, "surface_dim" => &mut self.surface_dim, "overlay0" => &mut self.overlay0, "overlay1" => &mut self.overlay1, "text" => &mut self.text, "subtext0" => &mut self.subtext0, "mauve" => &mut self.mauve,
            "green" => &mut self.green, "yellow" => &mut self.yellow, "red" => &mut self.red, "blue" => &mut self.blue, "teal" => &mut self.teal, "peach" => &mut self.peach, _ => return };
        *slot = v;
    }
    /// The selected row's background, as herdr's sidebar paints it: selection_bg, or active_row_bg when there is none.
    pub fn row(&self) -> C { if self.selection_bg == C::Own { self.active_row_bg } else { self.selection_bg } }
}
/// What the terminal told of itself: its appearance, and its own text and background colours.
#[derive(Clone, Copy, Debug, Default)]
pub struct Seen { pub light: Option<bool>, pub fg: Option<u32>, pub bg: Option<u32> }

/// herdr's config.toml, as far as the board reads it. A file that cannot be read gives herdr's defaults.
pub fn read_config() -> toml::Table { std::fs::read_to_string(crate::extras::herdr_config()).ok().and_then(|s| s.parse::<toml::Table>().ok()).unwrap_or_default() }
fn tbl<'a>(t: &'a toml::Table, k: &str) -> Option<&'a toml::Table> { t.get(k).and_then(|v| v.as_table()) }
fn st<'a>(t: Option<&'a toml::Table>, k: &str) -> Option<&'a str> { t.and_then(|t| t.get(k)).and_then(|v| v.as_str()) }
/// The dark and light pair a theme belongs to.
pub fn siblings(name: &str) -> (String, String) {
    const PAIRS: [(&str, &str); 7] = [("catppuccin", "catppuccin-latte"), ("tokyo-night", "tokyo-night-day"), ("gruvbox", "gruvbox-light"), ("one-dark", "one-light"), ("solarized", "solarized-light"), ("kanagawa", "kanagawa-lotus"), ("rose-pine", "rose-pine-dawn")];
    let c = canonical_theme_name(name).unwrap_or("");
    PAIRS.iter().find(|(d, l)| *d == c || *l == c).map(|(d, l)| (d.to_string(), l.to_string())).unwrap_or((name.into(), name.into()))
}
fn named(want: &str) -> Option<&'static str> { if ["herdr", "auto", "dark", "light"].contains(&want) { None } else { canonical_theme_name(want) } }
/// The theme herdr would use with this config. `light`: the terminal's appearance; `want`: the board's own
/// `board.theme` ("herdr" resolves the config as herdr does; "dark"/"light" pick by hand; a theme's name is that theme).
pub struct Resolved { pub name: String, pub pal: Pal, pub ground: C, pub symbols: bool }
pub fn resolve(cfg: &toml::Table, light: Option<bool>, want: &str, seen: Seen) -> Resolved {
    let (th, ui) = (tbl(cfg, "theme"), tbl(cfg, "ui"));
    let manual = named(want).map(String::from).unwrap_or_else(|| st(th, "name").unwrap_or("catppuccin").to_string());
    let (dark0, light0) = siblings(&manual);
    let auto = th.and_then(|t| t.get("auto_switch")).and_then(|v| v.as_bool()) == Some(true);
    let mode = if named(want).is_some() { "" } else if want == "dark" || want == "light" { want } else if auto { if light == Some(true) { "light" } else { "dark" } } else { "" };
    let name = match mode { "light" => st(th, "light_name").map(String::from).unwrap_or(light0), "dark" => st(th, "dark_name").map(String::from).unwrap_or(dark0), _ => manual };
    let base = Palette::from_name(&name).unwrap_or_else(|| if mode == "light" { Palette::catppuccin_latte() } else { Palette::catppuccin() });
    let canon = canonical_theme_name(&name).map(String::from).unwrap_or(name.clone());
    let mut pal = Pal::from(&base);
    let over = |pal: &mut Pal, t: Option<&toml::Table>| { if let Some(t) = t { for k in TOKENS { if let Some(v) = t.get(k).and_then(|v| v.as_str()) { pal.set(k, C::of(parse_color(v))); } } } };
    let custom = th.and_then(|t| tbl(t, "custom"));
    over(&mut pal, custom);
    // herdr's older accent setting.
    if let Some(a) = st(ui, "accent") { if a != "cyan" && st(custom, "accent").is_none() { pal.accent = C::of(parse_color(a)); } }
    if !mode.is_empty() && auto { over(&mut pal, custom.and_then(|c| tbl(c, mode))); }
    let terminal = Pal::from(&Palette::terminal());
    let own = |k: &str, now: C| canon == "terminal" && now == terminal.get(k);
    let ground = seen.bg.map(C::Rgb).unwrap_or(pal.panel_bg);
    let text = if pal.text != C::Own { pal.text } else { seen.fg.map(C::Rgb).unwrap_or(C::Own) };
    greys::readable_greys(&mut pal, &own, ground, text, seen.bg.is_some());
    pal.review = picks::review_blue(&pal);
    // The board's own check is neither the agent's nor the reviewer's: a neutral light grey, herdr's subtext0.
    pal.check = pal.subtext0;
    pal.todo = greys::todo_grey(&pal, ground, text);
    pal.auto = picks::auto_purple(&pal);
    pal.queue = picks::queue_pink(&pal, ground);
    let symbols = st(ui, "status_indicators") != Some("dots");
    Resolved { name: canon, pal, ground, symbols }
}

/// The theme the board draws with: the palette, the ground the greys were measured on, whether marks are herdr's
/// symbols or its dots, and whether colour is off (NO_COLOR).
#[derive(Clone, Debug)]
pub struct Theme { pub pal: Pal, pub ground: C, pub symbols: bool, pub plain: bool, pub name: String, pub web: bool }
impl Theme {
    pub fn load(light: Option<bool>, seen: Seen) -> Theme {
        let want = crate::store::settings::lookup::setting("board.theme", None).as_str().unwrap_or("herdr").to_string();
        let r = resolve(&read_config(), light, &want, seen);
        Theme { pal: r.pal, ground: r.ground, symbols: r.symbols, plain: no_color(), name: r.name, web: false }
    }
    /// The board's default look with no config read: herdr's own Catppuccin. For tests and frames.
    pub fn default_dark(plain: bool) -> Theme { let r = resolve(&toml::Table::new(), None, "herdr", Seen::default()); Theme { pal: r.pal, ground: r.ground, symbols: true, plain, name: r.name, web: false } }
    /// The web board's: the page gives each ink name a colour of its own, and draws each state's mark itself.
    pub fn web() -> Theme { Theme { web: true, ..Theme::default_dark(true) } }
    /// A theme for measuring only (every mark is one cell in every theme): made once.
    pub fn measure() -> &'static Theme { static T: std::sync::OnceLock<Theme> = std::sync::OnceLock::new(); T.get_or_init(|| Theme::default_dark(true)) }
    /// Does the terminal have to be asked for its appearance to finish the theme (herdr's auto_switch)?
    pub fn needs_light() -> bool {
        let want = crate::store::settings::lookup::setting("board.theme", None).as_str().unwrap_or("herdr").to_string();
        named(&want).is_none() && want != "dark" && want != "light" && tbl(&read_config(), "theme").and_then(|t| t.get("auto_switch")).and_then(|v| v.as_bool()) == Some(true)
    }
    pub fn ink(&self, i: Ink) -> C {
        let p = &self.pal;
        match i { Ink::Plain => C::Own, Ink::Text => p.text, Ink::Soft => p.subtext0, Ink::Muted => p.overlay0, Ink::Red => p.red, Ink::Green => p.green, Ink::Yellow => p.yellow, Ink::Teal => p.teal, Ink::Peach => p.peach,
            Ink::Mauve => p.mauve, Ink::Accent => p.accent, Ink::Review => p.review, Ink::Check => p.check, Ink::Queue => p.queue, Ink::Todo => p.todo, Ink::Auto => p.auto, Ink::Rule => p.surface1, Ink::Dim => p.surface_dim,
            Ink::Sidebar => p.sidebar_bg, Ink::Selection => p.row() }
    }
    /// A colour made quieter, for a mark that is finished with: measured, so it is still seen.
    pub fn quiet(&self, i: Ink) -> C { greys::quieter(self.ink(i), self.ground, self.pal.row()) }
    pub fn mark(&self, state: &str, kind: &str) -> &'static str { if self.web { marks::web_mark(state, kind) } else { marks::mark(state, kind, self.symbols) } }
    /// Can the selected row be painted? Where it cannot (colour off, no background in the theme) it is drawn reversed.
    pub fn paints_selection(&self) -> bool { !self.plain && self.pal.row() != C::Own }
}
pub fn no_color() -> bool { std::env::var("NO_COLOR").map(|v| !v.is_empty()).unwrap_or(false) }
pub use page::page_theme;

#[cfg(test)]
mod tests {
    use super::*;
    /// Every theme's greys and the board's own colours come out as the old board worked them out (tests/palettes.json
    /// was written by it), with and without the terminal telling its own colours.
    #[test]
    fn same_colours_as_before() {
        let want: serde_json::Value = serde_json::from_str(include_str!("../tests/palettes.json")).unwrap();
        let hex = |c: C| match c { C::Own => "own".to_string(), C::Ansi(n) => format!("ansi{n}"), C::Rgb(x) => format!("{x:06x}") };
        let mut bad = vec![];
        for (key, cols) in want.as_object().unwrap() {
            let (name, seen) = match key.strip_suffix("+seen") { Some(n) => (n, Seen { light: None, fg: Some(0xdddddd), bg: Some(0x101010) }), None => (key.as_str(), Seen::default()) };
            let r = resolve(&toml::Table::new(), None, name, seen);
            for (k, v) in cols.as_object().unwrap() { let got = hex(r.pal.get(k)); if got != v.as_str().unwrap() { bad.push(format!("{key} {k}: {got} != {v}")); } }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
