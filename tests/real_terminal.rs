//! [real programs] The board itself in a real terminal (a pseudo-terminal this test drives as a terminal would): it
//! starts and draws, takes a burst of keys, resizes cleanly, never clears the whole screen for a key, draws each frame
//! as one synchronized update, and on closing gives the terminal back exactly as it was: nothing left in your shell.
use crate::common;
use common::*;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::time::{Duration, Instant};

struct Term { master: std::fs::File, child: std::process::Child, out: Vec<u8>, answered: usize }
fn winsize(fd: i32, cols: u16, rows: u16) { let ws = libc::winsize { ws_row: rows, ws_col: cols, ws_xpixel: 0, ws_ypixel: 0 }; unsafe { libc::ioctl(fd, libc::TIOCSWINSZ as _, &ws) }; }
fn start(cwd: &std::path::Path, cols: u16, rows: u16) -> Term {
    let (mut m, mut s) = (0, 0);
    assert_eq!(unsafe { libc::openpty(&mut m, &mut s, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) }, 0);
    winsize(m, cols, rows);
    let slave = unsafe { OwnedFd::from_raw_fd(s) };
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_herdr-projects"));
    c.current_dir(cwd).env("TERM", "xterm-256color").env("COLORTERM", "truecolor").env("LANG", "C.UTF-8").env_remove("NO_COLOR")
        .env("HERDR_SOCKET_PATH", "/nonexistent").env("HERDR_BIN_PATH", "/bin/false")
        .stdin(slave.try_clone().unwrap()).stdout(slave.try_clone().unwrap()).stderr(slave);
    unsafe { c.pre_exec(|| { libc::setsid(); libc::ioctl(0, libc::TIOCSCTTY as _, 0); Ok(()) }); }
    let child = c.spawn().unwrap();
    unsafe { libc::fcntl(m, libc::F_SETFL, libc::fcntl(m, libc::F_GETFL) | libc::O_NONBLOCK) };
    Term { master: unsafe { std::fs::File::from_raw_fd(m) }, child, out: vec![], answered: 0 }
}
impl Term {
    /// What came out in the next `ms`, answering the terminal questions a terminal answers.
    fn read(&mut self, ms: u64) -> Vec<u8> {
        let mut got = vec![];
        let until = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < until {
            let mut b = [0u8; 65536];
            match self.master.read(&mut b) { Ok(n) if n > 0 => got.extend_from_slice(&b[..n]), _ => std::thread::sleep(Duration::from_millis(2)) }
        }
        self.out.extend_from_slice(&got);
        let tail = String::from_utf8_lossy(&self.out[self.answered..]).into_owned();
        if tail.contains("\x1b]10;?") { let _ = self.master.write_all(b"\x1b]10;rgb:cdcd/d6d6/f4f4\x1b\\\x1b]11;rgb:1e1e/1e1e/2e2e\x1b\\\x1b[?62;22c"); }
        self.answered = self.out.len();
        got
    }
    fn until(&mut self, what: &str, ms: u64) -> bool { let end = Instant::now() + Duration::from_millis(ms); while Instant::now() < end { self.read(20); if plain(&self.out).contains(what) { return true; } } false }
    /// What came out until a whole frame was drawn (its synchronized update ended), or `ms` passed with none.
    fn frame(&mut self, ms: u64) -> Vec<u8> { let mut got = vec![]; let end = Instant::now() + Duration::from_millis(ms); while Instant::now() < end && !has(&got, "\x1b[?2026l") { got.extend(self.read(5)); } got }
    fn keys(&mut self, b: &[u8]) -> Vec<u8> { self.master.write_all(b).unwrap(); self.frame(150) }
    fn resize(&mut self, cols: u16, rows: u16) -> Vec<u8> { use std::os::fd::AsRawFd; winsize(self.master.as_raw_fd(), cols, rows); self.frame(300) }
}
fn plain(b: &[u8]) -> String { regex::Regex::new(r"\x1b\[[0-9;?<>=]*[A-Za-z]|\x1b\][^\x07\x1b]*(\x07|\x1b\\)").unwrap().replace_all(&String::from_utf8_lossy(b), "").into_owned() }
fn has(b: &[u8], s: &str) -> bool { String::from_utf8_lossy(b).contains(s) }

#[test]
fn real_programs_the_board_starts_takes_keys_resizes_and_leaves_nothing_behind() {
    let w = world();
    for t in ["Write the notes", "Fix the footer", "Look at 日本語 🚀"] { w.add(t); }
    let t0 = Instant::now();
    let mut t = start(&w.root, 100, 30);
    assert!(t.until("herdr-projects", 5000), "the board drew nothing: {:?}", plain(&t.out));
    let first = t0.elapsed();
    eprintln!("first frame after {first:?}");
    assert!(first < Duration::from_secs(3), "the first frame took {first:?}");
    assert!(has(&t.out, "\x1b[?1049h"), "the board has a screen of its own");
    // a burst of keys: every frame one synchronized update, and no key clears the whole screen
    let mut burst = vec![];
    for _ in 0..9 { burst.extend(t.keys(b"\x1b[B")); }
    for k in ["h", "i", " ", "日", "本", " ", "👍"] { burst.extend(t.keys(k.as_bytes())); }
    // the terminal's own cursor stays hidden, and each frame leaves it where you type (an input method opens there)
    let typed = t.keys(b"!");
    assert!(regex::Regex::new(r"\x1b\[\d+;([3-9]|\d\d+)H\x1b\[\?2026l").unwrap().is_match(&String::from_utf8_lossy(&typed)), "{:?}", String::from_utf8_lossy(&typed));
    burst.extend(typed);
    assert!(!has(&burst, "\x1b[?25h"), "the cursor showed");
    burst.extend(t.keys(b"\x1b"));
    burst.extend(t.keys(b"?"));
    burst.extend(t.keys(b"\x1b"));
    assert!(!has(&burst, "\x1b[2J"), "a key cleared the whole screen");
    assert!(has(&burst, "\x1b[?2026h") && has(&burst, "\x1b[?2026l"), "frames are drawn as one update");
    // only the cells that change are drawn: a line low on the screen, under the list's rows, shows whole
    assert!(plain(&burst).contains("a page of tasks up or down"), "the ? menu showed");
    // resized: drawn again for the new size, also very small, and back
    for (c, r) in [(80, 24), (45, 12), (20, 6), (120, 40)] { let b = t.resize(c, r); assert!(!b.is_empty(), "nothing drawn at {c}×{r}"); }
    assert!(t.until("Fix the footer", 2000));
    // closing: ctrl+c twice; the terminal is given back as it was
    let before = t.out.len();
    t.keys(b"\x03"); t.keys(b"\x03");
    let end = Instant::now() + Duration::from_secs(5);
    let status = loop { if let Some(s) = t.child.try_wait().unwrap() { break s; } assert!(Instant::now() < end, "the board did not close"); t.read(20); };
    t.read(50);
    assert!(status.success(), "{status:?}");
    let tail = &t.out[before..];
    for (what, seq) in [("its own screen", "\x1b[?1049l"), ("the cursor", "\x1b[?25h"), ("bracketed paste", "\x1b[?2004l"), ("the mouse", "\x1b[?1000l")] { assert!(has(tail, seq), "{what} is not given back"); }
    // nothing is printed after the screen is given back
    let after = String::from_utf8_lossy(tail).rsplit("\x1b[?1049l").next().unwrap_or("").to_string();
    assert!(plain(after.as_bytes()).trim().is_empty(), "left in the shell: {after:?}");
}
