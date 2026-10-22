//! The board's own colours, picked to fit the theme: "review", the blue of a review that is running, "auto", the
//! purple of an automation, and "queue", the pink of a task that waits for a free slot. Each is the theme's own colour
//! of that hue where it is one and told apart from all it sits beside; else the first of the board's own that is; else
//! the one furthest from them.
use super::greys::{apart, rgb};
use super::{Pal, C};

pub const REVIEW_APART: f64 = 60.0;
pub const REVIEW_BLUE: u32 = 0x3b82f6;
const REVIEW_BLUES: [u32; 6] = [REVIEW_BLUE, 0x1d4ed8, 0x93c5fd, 0x0000ff, 0x6366f1, 0x0ea5e9];
/// Is it a blue at all? By its hue, from the blue-green side of cyan to the edge of violet, and not a grey.
pub fn is_blue(c: C) -> bool {
    let Some(x) = c.drawn() else { return false };
    let [r, g, b] = rgb(x);
    let spread = b - r.min(g);
    let hue = 60.0 * (4.0 + (r - g) / if spread == 0.0 { 1.0 } else { spread });
    b > r && b > g && spread >= 40.0 && (185.0..=255.0).contains(&hue)
}
fn least(c: C, beside: &[C]) -> f64 { beside.iter().map(|k| apart(c, *k)).fold(f64::INFINITY, f64::min) }
fn furthest(all: &[C], beside: &[C]) -> C { all.iter().copied().fold(None, |best: Option<C>, c| match best { Some(b) if least(b, beside) >= least(c, beside) => Some(b), _ => Some(c) }).unwrap_or(C::Own) }
/// "review": green says only "passed" or "ready" and amber only "the agent works", so a running review is a blue: the
/// theme's own where it is a blue told apart from green, red, amber, teal, the greys, text and background.
pub const REVIEW_BESIDE: [&str; 8] = ["green", "red", "yellow", "teal", "subtext0", "overlay0", "text", "panel_bg"];
pub fn review_blue(p: &Pal) -> C {
    let beside = REVIEW_BESIDE.map(|k| p.get(k));
    let fits = |c: C| is_blue(c) && least(c, &beside) >= REVIEW_APART;
    if let Some(c) = std::iter::once(p.blue).chain(REVIEW_BLUES.iter().map(|x| C::Rgb(*x))).find(|c| fits(*c)) { return c; }
    let mut any = vec![];
    for r in [0u32, 64, 128, 192] { for g in [0u32, 64, 128, 192] { for b in [160u32, 208, 255] { let c = C::Rgb(r << 16 | g << 8 | b); if is_blue(c) { any.push(c); } } } }
    furthest(&any, &beside)
}
pub const AUTO_APART: f64 = 40.0;
const PURPLES: [u32; 5] = [0xa855f7, 0xc084fc, 0x7c3aed, 0x6d28d9, 0xd946ef];
pub fn is_purple(c: C) -> bool {
    let Some(x) = c.drawn() else { return false };
    let [r, g, b] = rgb(x);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let spread = max - min;
    let s = if spread == 0.0 { 1.0 } else { spread };
    let h = if max == r { ((g - b) / s) % 6.0 } else if max == g { (b - r) / s + 2.0 } else { (r - g) / s + 4.0 };
    let hue = (60.0 * h + 360.0) % 360.0;
    g < r && g < b && spread >= 30.0 && (250.0..=345.0).contains(&hue)
}
fn is_light(c: u32) -> bool { let [r, g, b] = rgb(c); 0.2126 * r + 0.7152 * g + 0.0722 * b > 140.0 }
/// "auto": no task's state has it. The theme's own mauve where that is a purple told apart from every status colour,
/// the greys, text and background; among the terminal's own 16 colours, its magenta; else one of the board's purples.
pub const AUTO_BESIDE: [&str; 10] = ["red", "green", "yellow", "teal", "review", "todo", "text", "subtext0", "overlay0", "panel_bg"];
pub fn auto_purple(p: &Pal) -> C {
    let beside = AUTO_BESIDE.map(|k| p.get(k));
    let fits = |c: C| is_purple(c) && least(c, &beside) >= AUTO_APART;
    if fits(p.mauve) { return p.mauve; }
    if [p.mauve, p.text].iter().any(|c| c.real().is_none()) && fits(C::Ansi(5)) { return C::Ansi(5); }
    let light = p.panel_bg.real().map(is_light).unwrap_or(false);
    let own: Vec<C> = if light { PURPLES[2..].iter().chain(PURPLES.iter()).map(|x| C::Rgb(*x)).collect() } else { PURPLES.iter().map(|x| C::Rgb(*x)).collect() };
    own.iter().copied().find(|c| fits(*c)).unwrap_or_else(|| furthest(&own, &beside))
}

pub const QUEUE_APART: f64 = 55.0;
/// The board's pinks: for a dark ground the brighter first, for a light one the darker first.
const DARK_PINKS: [u32; 6] = [0xf472b6, 0xff6ec7, 0xec4899, 0xf9a8d4, 0xe040a0, 0xff9ad5];
const LIGHT_PINKS: [u32; 6] = [0xdb2777, 0xc2188b, 0xbe185d, 0xe040a0, 0x9d174d, 0xec4899];
/// Is it a pink: a red that leans to magenta, from hot pink to rose, and not a grey.
pub fn is_pink(c: C) -> bool {
    let Some(x) = c.drawn() else { return false };
    let [r, g, b] = rgb(x);
    let spread = r - g.min(b);
    let hue = (60.0 * ((g - b) / if spread == 0.0 { 1.0 } else { spread }) + 360.0) % 360.0;
    r >= g && r >= b && b > g && spread >= 50.0 && (300.0..=350.0).contains(&hue)
}
/// "queue": herdr's themes have no pink of their own, and no status of herdr's is one, so it is one of the board's
/// pinks, the first that is told apart from every status colour (red above all, the nearest to it), the automations'
/// purple, the greys, text and background, and is seen on the ground (3 to 1, what a mark needs). The `terminal`
/// theme's sixteen have no pink (their magenta is the automations'), so there too it is one of the board's, the one
/// that reads on a dark terminal and a light one alike.
pub const QUEUE_BESIDE: [&str; 11] = ["red", "green", "yellow", "teal", "review", "todo", "auto", "text", "subtext0", "overlay0", "panel_bg"];
pub fn queue_pink(p: &Pal, ground: C) -> C {
    let beside = QUEUE_BESIDE.map(|k| p.get(k));
    let reads = |c: C| match (c.real(), ground.real()) { (Some(x), Some(g)) => super::greys::contrast(x, g) >= super::greys::LEAST, _ => true };
    let fits = |c: C| is_pink(c) && reads(c) && least(c, &beside) >= QUEUE_APART;
    let light = ground.real().or(p.panel_bg.real()).map(is_light);
    // A ground that cannot be known (the terminal's own) takes the middle pink first, that reads on dark and light.
    let own: Vec<C> = match light { Some(true) => LIGHT_PINKS.to_vec(), Some(false) => DARK_PINKS.to_vec(), None => [0xec4899].into_iter().chain(DARK_PINKS).chain(LIGHT_PINKS).collect() }.into_iter().map(C::Rgb).collect();
    own.iter().copied().find(|c| fits(*c)).unwrap_or_else(|| { let seen: Vec<C> = own.iter().copied().filter(|c| reads(*c)).collect(); furthest(if seen.is_empty() { &own } else { &seen }, &beside) })
}
