//! The same theme for a page (the review page, the web board), as CSS variables. A page has no terminal behind it:
//! where herdr follows the terminal's appearance (auto_switch) or its colours (the "terminal" theme), the page follows
//! the browser's instead.
use super::{read_config, resolve, Resolved, Seen, Pal};
use crate::herdr::palette::Palette;

fn hex(c: u32) -> String { format!("#{c:06x}") }
fn is_light(c: u32) -> bool { let (r, g, b) = ((c >> 16) as f64, ((c >> 8) & 255) as f64, (c & 255) as f64); 0.2126 * r + 0.7152 * g + 0.0722 * b > 140.0 }
/// Every colour of a palette a page can name, by herdr's token (and the board's own).
pub fn vars(r: &Resolved, light: Option<bool>) -> (bool, Vec<(&'static str, String)>) {
    let terminal = Pal::from(&Palette::terminal());
    // A named colour the user chose is kept. The "terminal" theme's own colours, and "reset", mean whatever the
    // terminal shows, which a page cannot know: those take Catppuccin's, dark or light.
    let own = |k: &str| -> Option<u32> { let c = r.pal.get(k); if r.name == "terminal" && c == terminal.get(k) { return None; } c.drawn() };
    let light = light.unwrap_or_else(|| own("panel_bg").map(is_light).or_else(|| own("text").map(|t| !is_light(t))).unwrap_or(false));
    let base = Pal::from(&if light { Palette::catppuccin_latte() } else { Palette::catppuccin() });
    let v = |k: &str| hex(own(k).or_else(|| base.get(k).drawn()).unwrap_or(0));
    let keys = ["panel_bg", "text", "subtext0", "overlay0", "surface0", "surface1", "accent", "green", "red", "yellow", "teal", "peach", "mauve", "review", "check", "todo", "auto", "queue", "sidebar_bg", "selection_bg", "active_row_bg"];
    (light, keys.iter().map(|k| (*k, v(k))).collect())
}
fn block(r: &Resolved, light: Option<bool>) -> String {
    let (light, all) = vars(r, light);
    let get = |k: &str| all.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone()).unwrap_or_default();
    let mut s = format!("color-scheme:{};--bg:{};--ink:{};--muted:{};--line:{};--accent:{};--well:{};--ok:{};--bad:{}", if light { "light" } else { "dark" }, get("panel_bg"), get("text"), get("subtext0"), get("surface1"), get("accent"), get("surface0"), get("green"), get("red"));
    for (k, v) in &all { s.push_str(&format!(";--c-{}:{v}", k.replace('_', "-"))); }
    s
}
/// The board's colours for a page: one block, or a dark one with a light one for a browser that asks for light.
pub fn page_theme() -> String { page_theme_of(&read_config(), crate::store::settings::lookup::setting("board.theme", None).as_str().unwrap_or("herdr")) }
/// The same for this config of herdr's and this `board.theme`.
pub fn page_theme_of(cfg: &toml::Table, want: &str) -> String {
    let one = resolve(cfg, None, want, Seen::default());
    let by_hand = want == "dark" || want == "light";
    let auto = cfg.get("theme").and_then(|t| t.get("auto_switch")).and_then(|v| v.as_bool()) == Some(true);
    if by_hand || (!auto && one.name != "terminal") { return format!(":root{{{}}}", block(&one, if by_hand { Some(want == "light") } else { None })); }
    let dark = resolve(cfg, Some(false), want, Seen::default());
    let light = resolve(cfg, Some(true), want, Seen::default());
    format!(":root{{{}}}@media (prefers-color-scheme:light){{:root{{{}}}}}", block(&dark, Some(false)), block(&light, Some(true)))
}
