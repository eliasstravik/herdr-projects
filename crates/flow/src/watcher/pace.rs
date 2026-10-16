//! The Watcher's pace: how often it looks, how much it gives way when the machine is busy, and which screens count as
//! changed. Nothing here starts a program or reads a file: the look itself is watcher.rs.

/// The Watcher thinks about looking this often, not on every tick of the board.
pub const TICK_MS: i64 = 5000;
/// From one call's start to the next, at the least.
pub const GAP_MS: i64 = 15000;
/// Agents one call covers at the most; the rest ride in the next call.
pub const BATCH: usize = 8;

/// How busy the machine is: the one-minute load per core. 1 is every core in use, 2 is twice what it can do.
pub fn load_now() -> f64 {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
    match crate::os::load_average() { Some(l) => l[0] / cores.max(1.0), None => 0.0 }
}
/// The Watcher is the first to give way: up to three quarters of the cores in use it keeps its pace, a fully used
/// machine doubles the wait between looks, and from 1.75 on it looks five times less often.
pub fn slowdown(load: f64) -> f64 { if load > 0.75 { (1.0 + (load - 0.75) * 4.0).min(5.0) } else { 1.0 } }
/// A call that failed is not tried again at full pace: each failure in a row doubles the wait, up to eight times.
pub fn after_failures(fails: u32) -> f64 { 2f64.powi(fails as i32).min(8.0) }
/// The words on a screen, without what changes while nothing happens: a spinner's glyph, a timer, a token count.
/// Two screens with the same words are the same screen, and a second look at it would say what the first one said.
pub fn words_of(lines: &str) -> String {
    let only: String = lines.chars().map(|c| if c.is_alphabetic() || c == '\n' { c } else { ' ' }).collect();
    let mut out = vec![];
    for line in only.split('\n') {
        let kept: Vec<&str> = line.split(' ').filter(|w| w.chars().count() != 1).collect();
        out.push(kept.join(" "));
    }
    let joined = out.join("\n");
    let mut squeezed = String::new();
    for c in joined.chars() { if c == ' ' && squeezed.ends_with(' ') { continue; } squeezed.push(c); }
    squeezed.trim().to_string()
}
/// The last lines of a screen, as the Watcher is shown them.
pub fn last_lines(screen: &str) -> String {
    let lines: Vec<&str> = screen.split('\n').map(|l| l.trim_end()).filter(|l| !l.trim().is_empty()).collect();
    let s = lines[lines.len().saturating_sub(30)..].join("\n");
    let n = s.chars().count();
    s.chars().skip(n.saturating_sub(3000)).collect()
}
/// Who rides in the next call: (index, at) with `at` the time of each one's last look (0: never). Nobody is looked at
/// before its time (`every`) is up. A call takes everyone whose time is up, the longest wait first. So that agents
/// which started at different moments end up in one call, a call holds off while another agent is more than half-way
/// to its own time, but nobody waits more than half as long again.
pub fn riders(rows: &[(usize, i64)], now: i64, every: f64) -> Vec<usize> {
    let wait = |at: i64| (now - at) as f64;
    let mut due: Vec<(usize, i64)> = rows.iter().copied().filter(|r| wait(r.1) >= every).collect();
    if due.is_empty() { return vec![]; }
    let soon = rows.iter().any(|r| wait(r.1) >= every / 2.0 && wait(r.1) < every);
    if soon && !due.iter().any(|r| wait(r.1) >= every * 1.5) { return vec![]; }
    due.sort_by_key(|r| r.1);
    due.into_iter().map(|r| r.0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spinner_and_timers_do_not_count() {
        assert_eq!(words_of("⠋ Working (12s · 3.4k tokens)"), words_of("⠙ Working (13s · 3.5k tokens)"));
        assert_eq!(riders(&[(0, 0), (1, 20_000)], 60_000, 60_000.0), Vec::<usize>::new());
        assert_eq!(riders(&[(0, 0), (1, 1)], 61_000, 60_000.0), vec![0, 1]);
    }
}
