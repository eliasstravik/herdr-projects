//! The text box drawn within the rows the board gives it: a rule on top and under it that says how much of a long
//! draft is out of view (`↑ 3 more`), the prompt mark on the first line, the cursor as a reversed cell. A draft longer
//! than its rows scrolls inside them, never off the screen. A picture's chip (`[Image 1]`) is in the accent.
use super::TextBox;
use crate::shown::ink::{Ink, Inked};
use unicode_segmentation::UnicodeSegmentation;

/// A rule with what is out of view in its middle: `──── ↑ 3 more ────`.
pub fn scroll_rule(dir: &str, hidden: usize, width: usize) -> String {
    if hidden == 0 { return "─".repeat(width); }
    let label = format!(" {dir} {hidden} more ");
    let lw = crate::text::width(&label);
    if lw + 2 <= width { let left = (width - lw) / 2; return format!("{}{label}{}", "─".repeat(left), "─".repeat(width - left - lw)); }
    let ind = format!("─── {dir} {hidden} more ");
    let iw = crate::text::width(&ind);
    if iw <= width { return format!("{ind}{}", "─".repeat(width - iw)); }
    let dots = &"..."[..width.min(3)];
    format!("{}{dots}", crate::text::head(&ind, width.saturating_sub(dots.len())))
}
/// What the box draws: its lines (rule, body, rule) and where the cursor is among them (line, cell).
pub struct Drawn { pub lines: Vec<Inked>, pub cursor: Option<(usize, usize)> }
impl TextBox {
    /// `label`: the prompt ("❯"), on the first line; `tone`: the colour of the rules. `focused`: the cursor shows.
    pub fn render_within(&mut self, width: usize, max_rows: usize, label: &Inked, tone: Ink, focused: bool) -> Drawn {
        let lw = label.width() + 1;
        let height = max_rows.max(1);
        let content = width.saturating_sub(lw).max(1);
        // One cell is kept for the cursor at a line's end.
        let layout = content.saturating_sub(1).max(1);
        self.layout_width = layout;
        let v = self.visual(layout);
        let at = self.cursor_visual(&v);
        if at < self.viewport_top { self.viewport_top = at; }
        if at >= self.viewport_top + height { self.viewport_top = at + 1 - height; }
        self.viewport_top = self.viewport_top.min(v.len().saturating_sub(height));
        let shown = &v[self.viewport_top..(self.viewport_top + height).min(v.len())];
        self.above = self.viewport_top;
        self.below = v.len() - self.viewport_top - shown.len();
        let mut lines = vec![Inked::of(scroll_rule("↑", self.above, width), tone)];
        let mut cursor = None;
        for (k, (n, c)) in shown.iter().enumerate() {
            let line = self.lines.get(*n).map(|s| s.as_str()).unwrap_or("");
            let text = &line[c.start..c.end];
            let lead = if k == 0 { label.clone().add(Inked::plain(" ")) } else { Inked::spaces(lw) };
            let mut body = Inked::new();
            let here = self.viewport_top + k == at;
            if here && focused {
                let col = self.cursor().1.clamp(c.start, c.end) - c.start;
                let (before, after) = text.split_at(col);
                self.part(&mut body, *n, c.start, c.start + col);
                cursor = Some((lines.len(), lw + crate::text::width(before)));
                match after.graphemes(true).next() {
                    Some(g) => { let from = c.start + col + g.len(); body = body.add(Inked::plain(g).inv()); self.part(&mut body, *n, from, c.end); }
                    None => { body = body.add(Inked::plain(" ").inv()); }
                }
            } else { self.part(&mut body, *n, c.start, c.end); }
            lines.push(lead.add(body));
        }
        lines.push(Inked::of(scroll_rule("↓", self.below, width), tone));
        Drawn { lines, cursor }
    }
    /// Bytes `a..b` of line `n`, its chips in the accent.
    fn part(&self, body: &mut Inked, n: usize, a: usize, b: usize) {
        let Some(line) = self.lines.get(n) else { return };
        let mut at = a;
        for (s, e) in self.chips_of(n) {
            let (s, e) = (s.max(at), e.min(b));
            if s >= e { continue; }
            body.push(&line[at..s], Ink::Plain);
            body.push(&line[s..e], Ink::Accent);
            at = e;
        }
        body.push(&line[at..b], Ink::Plain);
    }
    /// A rule of another width that says the same as the last draw's.
    pub fn rule(&self, width: usize, up: bool) -> String { if up { scroll_rule("↑", self.above, width) } else { scroll_rule("↓", self.below, width) } }
}
