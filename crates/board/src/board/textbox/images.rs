//! Pictures pasted into the text box, shown as `[Image 1]`, `[Image 2]`: a picture's path pasted (the path herdr
//! pastes for a picture on its clipboard, a picture file dropped in the terminal, alone, several or among words) or
//! come as keys goes in as one chip per picture, numbered per message. A chip moves and deletes as one character; the
//! words name the path again, exactly as it was pasted, wherever they are read (`text()`: sent, kept as the draft).
//! What was sent shows its pictures as chips too (`labelled`). docs/image-paste.md.
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
/// A path with a picture's end: absolute or under `~/`, wherever its file is.
fn names_picture(token: &str) -> bool {
    let p = unescaped(token);
    (p.starts_with('/') || p.starts_with("~/")) && !p.contains('\n') && p.rsplit_once('.').is_some_and(|(_, end)| PICTURES.contains(&end.to_ascii_lowercase().as_str()))
}
/// herdr's own name for a picture it pasted, whole: nothing more comes after it.
fn is_herdrs(token: &str) -> bool { crate::re!(r"/herdr-clipboard-images-\d+/client-\d+-clipboard-\d+-\d+\.(png|jpe?g|gif|webp|bmp)$").is_match(&unescaped(token)) }
/// A picture's path: one with a picture's end that is herdr's own or a file there is.
pub fn is_picture(token: &str) -> bool {
    if !names_picture(token) { return false; }
    let p = unescaped(token);
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
/// Words already sent, each picture in them as its chip, numbered from 1: a path with a picture's end (the file may be
/// on the machine it was sent to; `@<path>` as the web board sends one), and the `[Image #39]` Claude Code writes for
/// a picture it took in.
pub fn labelled(text: &str) -> String {
    // A path at the end of a sentence: the mark after it is no part of it.
    let bare = |(a, b): (usize, usize)| (a, a + text[a..b].trim_end_matches(['.', ',', ';', ':', '!', '?', ')']).len());
    let mut at: Vec<(usize, usize)> = words(text).into_iter().map(bare).filter(|(a, b)| names_picture(text[*a..*b].trim_start_matches('@'))).collect();
    at.extend(crate::re!(r"\[Image #\d+\]").find_iter(text).map(|m| (m.start(), m.end())));
    at.sort();
    let (mut out, mut from, mut n) = (String::new(), 0, 0);
    for (a, b) in at {
        if a < from { continue; }
        n += 1;
        out.push_str(&text[from..a]);
        out.push_str(&label(n));
        from = b;
    }
    out.push_str(&text[from..]);
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
    /// Whether the next chip's label stands as words already, in the box (`around`) or in `text`: a chip by that name
    /// would make those words its path when sent, so the picture stays its path.
    fn taken(&self, around: &str, text: &str) -> bool { let l = label(self.images.len() + 1); around.contains(&l) || text.contains(&l) }
    /// `text` with every picture path in it as a chip, the pictures taken as this message's next. `around`: the words
    /// it goes in among.
    pub(super) fn collapse(&mut self, text: &str, around: &str) -> String {
        let ws = words(text);
        let pics: Vec<&(usize, usize)> = ws.iter().filter(|(a, b)| is_picture(&text[*a..*b])).collect();
        if pics.is_empty() { return text.to_string(); }
        let (mut out, mut at) = (String::new(), 0);
        for &&(a, b) in &pics {
            if self.taken(around, text) { break; }
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
    /// A paste with pictures' paths in it: their chips, with a space between a chip and the words on either side.
    pub(super) fn pasted(&mut self, s: &str) -> String {
        let had = self.images.len();
        let out = self.collapse(s.trim(), &self.raw());
        if self.images.len() == had { return s.to_string(); }
        let line = self.lines.get(self.line).map(|l| l.as_str()).unwrap_or("");
        let (lead, tail) = (&s[..s.len() - s.trim_start().len()], &s[s.trim_end().len()..]);
        let before = if !out.starts_with(&label(had + 1)) { lead } else if line[..self.col].chars().last().is_some_and(|c| !c.is_whitespace()) { " " } else { "" };
        let after = if !out.ends_with(&label(self.images.len())) { tail } else if line[self.col..].chars().next().is_some_and(char::is_whitespace) { "" } else { " " };
        format!("{before}{out}{after}")
    }
    /// A picture's path that came as keys, not as one paste (a terminal that does not mark a paste as one): its chip
    /// once a space is typed after it, or at once for herdr's own, whose name ends there; a space between it and a chip
    /// it was typed against. `c`: the key just typed.
    pub(super) fn typed(&mut self, c: char) {
        let line = self.cur().to_string();
        let end = if c.is_whitespace() { self.col - c.len_utf8() } else { self.col };
        // The word just before: from the end of a chip it touches, or where a shell would split.
        let chip = chips(&line[..end], self.images.len()).last().map(|c| c.1).unwrap_or(0);
        let Some((a, b)) = words(&line[chip..end]).last().map(|(a, b)| (a + chip, b + chip)) else { return };
        let word = &line[a..b];
        if b != end || !(c.is_whitespace() || is_herdrs(word)) || !is_picture(word) || self.taken(&self.raw(), "") { return; }
        self.images.push(word.to_string());
        // Never touching the chip before it.
        let gap = if line[..a].chars().last().is_some_and(|c| !c.is_whitespace()) { " " } else { "" };
        let chip = format!("{gap}{}", label(self.images.len()));
        self.lines[self.line].replace_range(a..b, &chip);
        self.col = self.col - (b - a) + chip.len();
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

