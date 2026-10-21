//! Pictures pasted into the text box, shown as `[Image 1]`, `[Image 2]`: a paste that is only paths of pictures (the
//! path herdr pastes for a picture on its clipboard, a picture file dropped in the terminal) goes in as one chip per
//! picture, numbered per message. A chip moves and deletes as one character; the words name the path again, exactly
//! as it was pasted, wherever they are read (`text()`: sent, kept as the draft). docs/image-paste.md.
use super::TextBox;

/// The ends of a picture's name.
const PICTURES: [&str; 9] = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "heic", "tif", "tiff"];
/// The folder herdr keeps a pasted picture in (`<temp>/herdr-clipboard-images-<uid>/client-…-clipboard-….png`).
const HERDR: &str = "/herdr-clipboard-images-";

pub fn label(n: usize) -> String { format!("[Image {n}]") }

/// A path as a shell gives it, without its quotes and `\` escapes.
fn unescaped(token: &str) -> String {
    let t = token.strip_prefix("file://").unwrap_or(token);
    if t.len() >= 2 && (t.starts_with('\'') && t.ends_with('\'') || t.starts_with('"') && t.ends_with('"')) { return t[1..t.len() - 1].to_string(); }
    let (mut out, mut esc) = (String::new(), false);
    for c in t.chars() { if esc { out.push(c); esc = false; } else if c == '\\' { esc = true; } else { out.push(c); } }
    out
}
/// A picture's path: absolute or under `~/`, with a picture's end, and herdr's own or a file there is.
pub fn is_picture(token: &str) -> bool {
    let p = unescaped(token);
    if !(p.starts_with('/') || p.starts_with("~/")) || p.contains('\n') { return false; }
    let Some((_, end)) = p.rsplit_once('.') else { return false };
    if !PICTURES.contains(&end.to_ascii_lowercase().as_str()) { return false; }
    if p.contains(HERDR) { return true; }
    let path = match p.strip_prefix("~/") { Some(r) => crate::store::files::user_home().join(r), None => p.into() };
    path.is_file()
}
/// The words of `text` split where a shell splits them: at whitespace not escaped with `\` nor inside quotes. (start,
/// end) in bytes.
fn words(text: &str) -> Vec<(usize, usize)> {
    let (mut out, mut from, mut esc, mut quote) = (vec![], None::<usize>, false, None::<char>);
    for (i, c) in text.char_indices() {
        if esc { esc = false; continue; }
        match quote {
            Some(q) => { if c == q { quote = None; } continue; }
            None if c == '\\' => { esc = true; from.get_or_insert(i); continue; }
            None if (c == '\'' || c == '"') && from.is_none() => { quote = Some(c); from = Some(i); continue; }
            None => {}
        }
        if c.is_whitespace() { if let Some(f) = from.take() { out.push((f, i)); } } else { from.get_or_insert(i); }
    }
    if let Some(f) = from { out.push((f, text.len())); }
    out
}
/// Where a chip of a picture this box knows stands in `line` (start, end), in bytes.
fn chips(line: &str, known: usize) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut at = 0;
    while let Some(i) = line[at..].find("[Image ") {
        let start = at + i;
        let digits: String = line[start + 7..].chars().take_while(char::is_ascii_digit).collect();
        let end = start + 7 + digits.len();
        at = start + 1;
        if digits.is_empty() || !line[end..].starts_with(']') { continue; }
        if digits.parse::<usize>().is_ok_and(|n| n >= 1 && n <= known) { out.push((start, end + 1)); at = end + 1; }
    }
    out
}

impl TextBox {
    /// `text` with every picture path in it as a chip, the pictures taken as this message's next. `only`: only when
    /// all of it is pictures' paths (a paste), else it stays as it is.
    pub(super) fn collapse(&mut self, text: &str, only: bool) -> String {
        let ws = words(text);
        let pics: Vec<&(usize, usize)> = ws.iter().filter(|(a, b)| is_picture(&text[*a..*b])).collect();
        if pics.is_empty() || (only && pics.len() < ws.len()) { return text.to_string(); }
        let (mut out, mut at) = (String::new(), 0);
        for &&(a, b) in &pics {
            out.push_str(&text[at..a]);
            self.images.push(text[a..b].to_string());
            out.push_str(&label(self.images.len()));
            at = b;
        }
        out.push_str(&text[at..]);
        out
    }
    /// The words with each chip as the path it stands for.
    pub(super) fn expand(&self, text: &str) -> String {
        let (mut out, mut at) = (String::new(), 0);
        for (a, b) in chips(text, self.images.len()) {
            out.push_str(&text[at..a]);
            let n: usize = text[a + 7..b - 1].parse().unwrap_or(0);
            out.push_str(&self.images[n - 1]);
            at = b;
        }
        out.push_str(&text[at..]);
        out
    }
    /// A paste of pictures only: their chips, with a space between them and the words on either side.
    pub(super) fn pasted(&mut self, s: &str) -> String {
        let had = self.images.len();
        let out = self.collapse(s.trim(), true);
        if self.images.len() == had { return s.to_string(); }
        let line = self.lines.get(self.line).map(|l| l.as_str()).unwrap_or("");
        let before = if line[..self.col].chars().last().is_some_and(|c| !c.is_whitespace()) { " " } else { "" };
        let after = if line[self.col..].chars().next().is_some_and(char::is_whitespace) { "" } else { " " };
        format!("{before}{out}{after}")
    }
    /// The start of the chip that ends at `col` on the cursor's line.
    pub(super) fn chip_before(&self, col: usize) -> Option<usize> { chips(self.cur(), self.images.len()).into_iter().find(|c| c.1 == col).map(|c| c.0) }
    /// The end of the chip that starts at `col` on the cursor's line.
    pub(super) fn chip_after(&self, col: usize) -> Option<usize> { chips(self.cur(), self.images.len()).into_iter().find(|c| c.0 == col).map(|c| c.1) }
    /// The cursor never stands inside a chip: it goes to its end.
    pub(super) fn out_of_chip(&mut self) {
        if let Some(c) = chips(self.cur(), self.images.len()).into_iter().find(|c| c.0 < self.col && self.col < c.1) { self.col = c.1; }
    }
    /// The chips of the line (start, end), to draw them.
    pub fn chips_of(&self, line: usize) -> Vec<(usize, usize)> { self.lines.get(line).map(|l| chips(l, self.images.len())).unwrap_or_default() }
}

