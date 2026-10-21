//! The box at the bottom you type in: the board's own, as no Ratatui text box wraps words, keeps undo and a kill ring
//! and edits by written characters (a letter with its marks, an emoji, a flag) the way pi-tui's editor did, which the
//! board's keys and docs/keys.md assume. Lines of text, a cursor, undo; keys.rs says what each key does, view.rs draws
//! it within the room the board gives it. A paste comes in whole and is never taken for keys; a picture's path pasted
//! stands as `[Image 1]` (images.rs).
pub mod images;
pub mod keys;
pub mod view;
pub mod wrap;

use crate::text::grapheme_width;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, PartialEq)]
struct Snap { lines: Vec<String>, line: usize, col: usize, images: Vec<String> }
#[derive(Clone, Debug, Default)]
pub struct TextBox {
    lines: Vec<String>, line: usize, col: usize,
    /// The paths of this message's pictures, as pasted: `[Image n]` stands for the n-th.
    images: Vec<String>,
    undo: Vec<Snap>, typing: bool,
    kill: Vec<String>, killed_last: bool, yank_at: Option<(usize, usize, usize)>,
    /// The column up and down keep to across lines of other lengths.
    pref: Option<usize>,
    /// The width the last draw wrapped at, which up and down move through.
    pub layout_width: usize,
    /// How much of the draft the last draw left out, above and below.
    pub above: usize, pub below: usize, pub viewport_top: usize,
}
impl TextBox {
    /// The words as they are sent: each picture's chip is its path again.
    pub fn text(&self) -> String { self.expand(&self.raw()) }
    /// The words as the box shows them, with the chips.
    pub fn raw(&self) -> String { if self.lines.is_empty() { String::new() } else { self.lines.join("\n") } }
    pub fn is_empty(&self) -> bool { self.lines.iter().all(|l| l.is_empty()) }
    fn ensure(&mut self) { if self.lines.is_empty() { self.lines.push(String::new()); } }
    fn snap(&self) -> Snap { Snap { lines: self.lines.clone(), line: self.line, col: self.col, images: self.images.clone() } }
    fn push_undo(&mut self) { let s = self.snap(); if self.undo.last() != Some(&s) { self.undo.push(s); if self.undo.len() > 200 { self.undo.remove(0); } } }
    /// Replace the whole text; the cursor goes to its end. Undoable, as a change made by the board. It is a new
    /// message: the pictures' paths in it are its chips, numbered from 1.
    pub fn set_text(&mut self, s: &str) {
        let norm = normalize(s);
        if norm != self.text() { self.push_undo(); }
        // What was cut stays what it said, chips or not.
        let kill: Vec<String> = self.kill.iter().map(|k| self.expand(k)).collect();
        self.kill = kill;
        self.images.clear();
        let norm = self.collapse(&norm, false);
        self.lines = norm.split('\n').map(String::from).collect();
        self.line = self.lines.len() - 1; self.col = self.lines[self.line].len();
        self.typing = false; self.pref = None; self.killed_last = false; self.yank_at = None;
        if norm.is_empty() { self.viewport_top = 0; }
    }
    pub fn cursor(&self) -> (usize, usize) { (self.line, self.col) }
    fn cur(&self) -> &str { self.lines.get(self.line).map(|s| s.as_str()).unwrap_or("") }
    /// Text typed or pasted at the cursor. A paste is one step of undo; one of pictures' paths only is their chips.
    pub fn insert(&mut self, s: &str) {
        let s = normalize(s);
        if s.is_empty() { return; }
        self.push_undo();
        self.ensure();
        let s = self.pasted(&s);
        self.insert_raw(&s);
        self.typing = false;
    }
    fn insert_raw(&mut self, s: &str) {
        self.ensure();
        let rest = self.lines[self.line].split_off(self.col);
        let mut parts = s.split('\n');
        if let Some(first) = parts.next() { self.lines[self.line].push_str(first); }
        for p in parts { self.line += 1; self.lines.insert(self.line, p.to_string()); }
        self.col = self.lines[self.line].len();
        self.lines[self.line].push_str(&rest);
        self.pref = None; self.killed_last = false; self.yank_at = None;
    }
    /// One typed character. Undo takes back a word at a time.
    pub fn type_char(&mut self, c: char) {
        if c.is_whitespace() || !self.typing { self.push_undo(); }
        self.typing = true;
        self.insert_raw(&c.to_string());
    }
    pub fn newline(&mut self) { self.push_undo(); self.typing = false; self.insert_raw("\n"); }
    pub fn undo(&mut self) { if let Some(s) = self.undo.pop() { self.lines = s.lines; self.line = s.line; self.col = s.col; self.images = s.images; } self.typing = false; self.pref = None; }

    // ---- moving ------------------------------------------------------------------------------------------------------
    // A chip is one character to these.
    fn prev_g(&self) -> usize { self.chip_before(self.col).unwrap_or_else(|| self.cur()[..self.col].grapheme_indices(true).last().map(|(i, _)| i).unwrap_or(0)) }
    fn next_g(&self) -> usize { let l = self.cur(); self.chip_after(self.col).unwrap_or_else(|| l[self.col..].graphemes(true).next().map(|g| self.col + g.len()).unwrap_or(l.len())) }
    pub fn left(&mut self) { self.pref = None; if self.col > 0 { self.col = self.prev_g(); } else if self.line > 0 { self.line -= 1; self.col = self.cur().len(); } }
    pub fn right(&mut self) { self.pref = None; if self.col < self.cur().len() { self.col = self.next_g(); } else if self.line + 1 < self.lines.len() { self.line += 1; self.col = 0; } }
    pub fn home(&mut self) { self.pref = None; self.col = 0; }
    pub fn end(&mut self) { self.pref = None; self.col = self.cur().len(); }
    fn word_left_at(&self) -> usize {
        let gs: Vec<(usize, &str)> = self.cur()[..self.col].grapheme_indices(true).collect();
        let mut n = gs.len();
        while n > 0 && gs[n - 1].1.chars().all(char::is_whitespace) { n -= 1; }
        if let Some(c) = self.chip_before(gs.get(n).map(|g| g.0).unwrap_or(self.col)) { return c; }
        let punct = n > 0 && is_punct(gs[n - 1].1);
        while n > 0 && !gs[n - 1].1.chars().all(char::is_whitespace) && is_punct(gs[n - 1].1) == punct { n -= 1; }
        gs.get(n).map(|g| g.0).unwrap_or(if n == 0 { 0 } else { self.col })
    }
    fn word_right_at(&self) -> usize {
        let l = self.cur();
        let gs: Vec<(usize, &str)> = l[self.col..].grapheme_indices(true).map(|(i, g)| (self.col + i, g)).collect();
        let mut n = 0;
        while n < gs.len() && gs[n].1.chars().all(char::is_whitespace) { n += 1; }
        if let Some(c) = self.chip_after(gs.get(n).map(|g| g.0).unwrap_or(l.len())) { return c; }
        let punct = n < gs.len() && is_punct(gs[n].1);
        while n < gs.len() && !gs[n].1.chars().all(char::is_whitespace) && is_punct(gs[n].1) == punct { n += 1; }
        gs.get(n).map(|g| g.0).unwrap_or(l.len())
    }
    pub fn word_left(&mut self) { self.pref = None; if self.col == 0 { return self.left(); } self.col = self.word_left_at(); }
    pub fn word_right(&mut self) { self.pref = None; if self.col == self.cur().len() { return self.right(); } self.col = self.word_right_at(); }

    // ---- deleting ----------------------------------------------------------------------------------------------------
    pub fn backspace(&mut self) {
        if self.col == 0 && self.line == 0 { return; }
        self.push_undo(); self.typing = false; self.pref = None;
        if self.col > 0 { let from = self.prev_g(); self.lines[self.line].replace_range(from..self.col, ""); self.col = from; }
        else { let l = self.lines.remove(self.line); self.line -= 1; self.col = self.lines[self.line].len(); self.lines[self.line].push_str(&l); }
    }
    pub fn delete(&mut self) {
        if self.col == self.cur().len() && self.line + 1 >= self.lines.len() { return; }
        self.push_undo(); self.typing = false; self.pref = None;
        if self.col < self.cur().len() { let to = self.next_g(); self.lines[self.line].replace_range(self.col..to, ""); }
        else { let l = self.lines.remove(self.line + 1); self.lines[self.line].push_str(&l); }
    }
    fn killed(&mut self, s: String, before: bool) {
        if s.is_empty() { return; }
        if self.killed_last { if let Some(last) = self.kill.last_mut() { if before { *last = format!("{s}{last}"); } else { last.push_str(&s); } } } else { self.kill.push(s); if self.kill.len() > 30 { self.kill.remove(0); } }
        self.killed_last = true;
    }
    pub fn delete_word_back(&mut self) {
        if self.col == 0 { return self.backspace(); }
        self.push_undo(); self.typing = false;
        let from = self.word_left_at();
        let s: String = self.lines[self.line].drain(from..self.col).collect();
        self.col = from; let was = self.killed_last; self.killed_last = was; self.killed(s, true);
    }
    pub fn delete_word_forward(&mut self) {
        if self.col == self.cur().len() { return self.delete(); }
        self.push_undo(); self.typing = false;
        let to = self.word_right_at();
        let s: String = self.lines[self.line].drain(self.col..to).collect();
        self.killed(s, false);
    }
    pub fn delete_to_start(&mut self) {
        if self.col == 0 { if self.line > 0 { self.backspace(); } return; }
        self.push_undo(); self.typing = false;
        let s: String = self.lines[self.line].drain(..self.col).collect();
        self.col = 0; self.killed(s, true);
    }
    pub fn delete_to_end(&mut self) {
        if self.col == self.cur().len() { if self.line + 1 < self.lines.len() { self.delete(); } return; }
        self.push_undo(); self.typing = false;
        let s: String = self.lines[self.line].drain(self.col..).collect();
        self.killed(s, false);
    }
    pub fn yank(&mut self) {
        let Some(s) = self.kill.last().cloned() else { return };
        self.push_undo(); self.typing = false;
        let (line, col) = (self.line, self.col);
        self.insert_raw(&s);
        self.yank_at = Some((line, col, self.kill.len() - 1));
    }
    pub fn yank_pop(&mut self) {
        let Some((line, col, n)) = self.yank_at else { return };
        if self.kill.len() < 2 { return; }
        let len = self.kill[n].len();
        if line != self.line || self.kill[n].contains('\n') { return; }
        self.lines[line].replace_range(col..col + len, "");
        self.col = col;
        let m = (n + self.kill.len() - 1) % self.kill.len();
        let s = self.kill[m].clone();
        self.insert_raw(&s);
        self.yank_at = Some((line, col, m));
    }
    /// The cursor to the next (or previous) place this character stands.
    pub fn jump(&mut self, c: char, forward: bool) {
        let all = self.raw();
        let at: usize = self.lines[..self.line].iter().map(|l| l.len() + 1).sum::<usize>() + self.col;
        let found = if forward { all[(at + 1).min(all.len())..].find(c).map(|i| at + 1 + i) } else { all[..at].rfind(c) };
        if let Some(i) = found { let mut n = i; for (k, l) in self.lines.iter().enumerate() { if n <= l.len() { self.line = k; self.col = n; break; } n -= l.len() + 1; } self.out_of_chip(); }
    }
    // ---- up and down, through the lines as they wrap ---------------------------------------------------------------
    /// The visual lines: (logical line, chunk).
    pub fn visual(&self, width: usize) -> Vec<(usize, wrap::Chunk)> {
        let mut out = vec![];
        for (n, l) in self.lines.iter().enumerate() { for c in wrap::word_wrap_line(l, width) { out.push((n, c)); } }
        if out.is_empty() { out.push((0, wrap::Chunk { start: 0, end: 0 })); }
        out
    }
    /// Which visual line the cursor is on.
    pub fn cursor_visual(&self, v: &[(usize, wrap::Chunk)]) -> usize {
        let mut last = 0;
        for (i, (n, c)) in v.iter().enumerate() {
            if *n != self.line { continue; }
            last = i;
            let is_last = v.get(i + 1).map(|x| x.0 != *n).unwrap_or(true);
            if self.col >= c.start && (self.col < c.end || is_last) { return i; }
        }
        last
    }
    /// Up (-1) or down (1) one visual line. False at the top or the bottom.
    pub fn vertical(&mut self, d: i64) -> bool {
        let w = self.layout_width.max(1);
        let v = self.visual(w);
        let at = self.cursor_visual(&v);
        let to = at as i64 + d;
        if to < 0 || to as usize >= v.len() { return false; }
        let (n, c) = &v[at];
        let col_now = crate::text::width(&self.lines[*n][c.start..self.col]);
        let want = *self.pref.get_or_insert(col_now);
        let (tn, tc) = &v[to as usize];
        let line = &self.lines[*tn];
        let mut col = tc.start;
        let mut w = 0;
        for (i, g) in line[tc.start..tc.end].grapheme_indices(true) { let gw = grapheme_width(g); if w + gw > want { break; } w += gw; col = tc.start + i + g.len(); }
        // On a line that goes on below, the cursor never stands past the end of its chunk.
        let more = v.get(to as usize + 1).map(|x| x.0 == *tn).unwrap_or(false);
        if more && col >= tc.end && tc.end > tc.start { col = line[tc.start..tc.end].grapheme_indices(true).last().map(|(i, _)| tc.start + i).unwrap_or(tc.start); }
        self.line = *tn; self.col = col;
        self.out_of_chip();
        true
    }
}
fn is_punct(g: &str) -> bool { g.chars().all(|c| "(){}[]<>.,;:'\"!?+-=*/\\|&%^$#@~`".contains(c)) }
/// Line ends as one, tabs as four spaces, and nothing a terminal would act on.
pub fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n").replace('\t', "    ").chars().filter(|c| *c == '\n' || !((*c as u32) < 0x20 || (0x7f..=0x9f).contains(&(*c as u32)))).collect()
}
