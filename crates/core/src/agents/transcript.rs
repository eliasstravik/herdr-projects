//! Where a program saved a conversation, found once and remembered: the keeper asks for every working task on every
//! tick, and a search of a large sessions folder each time costs most of a core.
use super::adapter::{dashed, real};
use super::homes::*;
use super::read_turn::find_file;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// How long a remembered answer is trusted before the search runs again, so a newer copy is picked up in time.
const FRESH: Duration = Duration::from_secs(300);
/// How long "not saved anywhere" is believed: a task whose conversation is gone is asked for every tick, and each miss
/// searches every folder. One that appears later is found within this time.
const MISS_FRESH: Duration = Duration::from_secs(30);
/// How many answers are kept; a long-running keeper sees many conversations come and go.
const CAP: usize = 1024;
/// Whether misses are remembered: only in the keeper, which asks every second; a board, a hook or a test asks
/// once and wants what is saved now (a hook can come just before its conversation is written).
static MISSES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn remember_misses() { MISSES.store(true, std::sync::atomic::Ordering::Relaxed); }
static FOUND: LazyLock<Mutex<HashMap<(String, String), (Option<PathBuf>, Instant)>>> = LazyLock::new(Default::default);

/// `a/./b/../c/` becomes `a/c`, without following links, as pi's path.resolve does.
fn lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c { Component::CurDir => {}, Component::ParentDir => { out.pop(); }, c => out.push(c) }
    }
    out
}
/// The folder pi saves a directory's conversations in: one leading separator dropped, then `/`, `\\` and `:` become `-`.
pub(super) fn pi_dir(cwd: &Path) -> String {
    let s = cwd.display().to_string();
    let s = s.strip_prefix(['/', '\\']).unwrap_or(&s);
    format!("--{}--", s.replace(['/', '\\', ':'], "-"))
}
/// Where a conversation is saved. The answer is remembered for a while and dropped early once the file is gone.
pub fn transcript(kind: &str, session: &str, cwd: &Path) -> Option<PathBuf> {
    let key = (kind.to_string(), session.to_string());
    let known = FOUND.lock().unwrap().get(&key).cloned();
    match known {
        Some((Some(f), at)) if at.elapsed() < FRESH && f.is_file() => return Some(f),
        // A miss is believed only after the places a new conversation is saved first are looked at again (cheap): a
        // hook can come before its conversation is written, and a coordinator looks before its first prompt.
        Some((None, at)) if at.elapsed() < MISS_FRESH && MISSES.load(std::sync::atomic::Ordering::Relaxed) => {
            let f = quick(kind, session, cwd)?;
            FOUND.lock().unwrap().insert(key, (Some(f.clone()), Instant::now()));
            return Some(f);
        }
        _ => {}
    }
    let found = locate(kind, session, cwd);
    let mut map = FOUND.lock().unwrap();
    if map.len() >= CAP { map.retain(|_, (f, at)| at.elapsed() < if f.is_some() { FRESH } else { MISS_FRESH }); }
    if map.len() >= CAP { map.clear(); }
    map.insert(key, (found.clone(), Instant::now()));
    found
}
/// pi's conversations for a folder, newest first: only the folders pi saves it under (as given, its real path, and
/// where it was before a move), unless none of them is there yet, then every folder.
pub(super) fn pi_saved(cwd: &Path, was: Option<&Path>) -> Vec<PathBuf> {
    let root = pi_home().join("sessions");
    let given = lexical(cwd);
    let mut dirs: Vec<PathBuf> = [Some(given.clone()), Some(real(&given)), was.map(lexical)].into_iter().flatten().map(|d| root.join(pi_dir(&d))).collect();
    dirs.sort();
    dirs.dedup();
    dirs.retain(|d| d.is_dir());
    let jsonl = |n: &str| n.ends_with(".jsonl");
    if dirs.is_empty() { return find_file(&root, &jsonl, 1); }
    let mut all: Vec<PathBuf> = dirs.iter().flat_map(|d| find_file(d, &jsonl, 0)).collect();
    all.sort_by_cached_key(|p| std::cmp::Reverse(std::fs::metadata(p).and_then(|m| m.modified()).ok()));
    all
}
/// The newest of a folder's sub-folders by name (codex saves under `YYYY/MM/DD`).
fn last_dir(d: &Path) -> Option<PathBuf> {
    std::fs::read_dir(d).ok()?.flatten().filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).map(|e| e.path()).max()
}
/// Where a new conversation is saved, looked at without a search: the folder for the cwd (claude, pi) or today's
/// folder (codex).
fn quick(kind: &str, session: &str, cwd: &Path) -> Option<PathBuf> {
    match kind {
        "claude" => Some(claude_home().join("projects").join(dashed(&real(cwd))).join(format!("{session}.jsonl"))).filter(|f| f.is_file()),
        "codex" => {
            let want = format!("{session}.jsonl");
            let day = last_dir(&last_dir(&last_dir(&codex_home().join("sessions"))?)?)?;
            find_file(&day, &|n| n.ends_with(&want), 0).into_iter().next()
        }
        "pi" => {
            let want = format!("_{session}.jsonl");
            let (root, given) = (pi_home().join("sessions"), lexical(cwd));
            [given.clone(), real(&given)].iter().find_map(|d| find_file(&root.join(pi_dir(d)), &|n| n.ends_with(&want), 0).into_iter().next())
        }
        _ => None,
    }
}
/// Every place a program may have saved a conversation: a search of its whole sessions folder.
fn locate(kind: &str, session: &str, cwd: &Path) -> Option<PathBuf> {
    if let Some(f) = quick(kind, session, cwd) { return Some(f); }
    match kind {
        "claude" => { let want = format!("{session}.jsonl"); find_file(&claude_home().join("projects"), &|n| n == want, 1).into_iter().next() }
        "codex" => { let want = format!("{session}.jsonl"); find_file(&codex_home().join("sessions"), &|n| n.ends_with(&want), 4).into_iter().next() }
        "pi" => { let want = format!("_{session}.jsonl"); find_file(&pi_home().join("sessions"), &|n| n.ends_with(&want), 1).into_iter().next() }
        _ => None,
    }
}

#[cfg(test)]
mod transcript_tests {
    use super::*;
    use std::fs;
    /// Tests share the process environment and the memo, so they take turns.
    static TURN: Mutex<()> = Mutex::new(());
    struct Home { root: PathBuf, was: Option<std::ffi::OsString>, misses: bool, _turn: std::sync::MutexGuard<'static, ()> }
    impl Drop for Home {
        fn drop(&mut self) {
            unsafe { match &self.was { Some(v) => std::env::set_var("PI_CODING_AGENT_DIR", v), None => std::env::remove_var("PI_CODING_AGENT_DIR") } }
            FOUND.lock().unwrap().clear();
            MISSES.store(self.misses, std::sync::atomic::Ordering::Relaxed);
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn home(name: &str, dirs: usize) -> Home {
        let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let root = std::env::temp_dir().join(format!("hp-pi-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for i in 0..dirs { let d = root.join(format!("sessions/--tmp-other-{i}--")); fs::create_dir_all(&d).unwrap(); fs::write(d.join(format!("2026_x{i}.jsonl")), "").unwrap(); }
        fs::create_dir_all(root.join("sessions")).unwrap();
        let was = std::env::var_os("PI_CODING_AGENT_DIR");
        unsafe { std::env::set_var("PI_CODING_AGENT_DIR", &root); }
        FOUND.lock().unwrap().clear();
        Home { root, was, misses: MISSES.load(std::sync::atomic::Ordering::Relaxed), _turn: turn }
    }
    fn save(h: &Home, dir: &str, session: &str) -> PathBuf {
        let d = h.root.join("sessions").join(dir);
        fs::create_dir_all(&d).unwrap();
        let f = d.join(format!("2026_{session}.jsonl"));
        fs::write(&f, "").unwrap();
        f
    }
    #[test]
    fn pi_dir_matches_pi_layout() {
        assert_eq!(pi_dir(Path::new("/Users/luke")), "--Users-luke--");
        assert_eq!(pi_dir(Path::new("C:\\work\\a b")), "--C--work-a b--");
        assert_eq!(pi_dir(&lexical(Path::new("/a/./b/../c/"))), "--a-c--");
    }
    #[test]
    fn found_in_its_own_folder_then_remembered() {
        let h = home("hit", 20);
        let mine = save(&h, "--tmp-hp-project--", "sw-t");
        assert_eq!(transcript("pi", "sw-t", Path::new("/tmp/hp-project")), Some(mine.clone()));
        save(&h, "--tmp-aaa-first--", "sw-t");
        fs::remove_dir_all(h.root.join("sessions/--tmp-hp-project--")).unwrap();
        fs::create_dir_all(&mine).unwrap();
        assert_eq!(transcript("pi", "sw-t", Path::new("/tmp/hp-project")).as_deref(), Some(h.root.join("sessions/--tmp-aaa-first--/2026_sw-t.jsonl").as_path()), "a folder where the file was is not taken as the file");
        assert_eq!(transcript("pi", "sw-absent", Path::new("/tmp/hp-project")), None);
    }
    #[test]
    fn a_miss_is_remembered_briefly() {
        let h = home("miss", 0);
        remember_misses();
        assert_eq!(transcript("pi", "sw-late", Path::new("/tmp/hp-project")), None);
        let f = save(&h, "--tmp-elsewhere--", "sw-late");
        assert_eq!(transcript("pi", "sw-late", Path::new("/tmp/hp-project")), None, "a miss is not searched for everywhere at once");
        FOUND.lock().unwrap().get_mut(&("pi".to_string(), "sw-late".to_string())).unwrap().1 -= MISS_FRESH;
        assert_eq!(transcript("pi", "sw-late", Path::new("/tmp/hp-project")), Some(f), "and is searched again after a while");
        assert_eq!(transcript("pi", "sw-new", Path::new("/tmp/hp-project")), None);
        let own = save(&h, "--tmp-hp-project--", "sw-new");
        assert_eq!(transcript("pi", "sw-new", Path::new("/tmp/hp-project")), Some(own), "one saved in its own folder is found at once");
    }
    #[test]
    fn a_remembered_answer_is_used_without_searching() {
        let h = home("memo", 0);
        let first = save(&h, "--tmp-elsewhere--", "sw-m");
        assert_eq!(transcript("pi", "sw-m", Path::new("/tmp/hp-project")), Some(first.clone()));
        save(&h, "--tmp-hp-project--", "sw-m");
        assert_eq!(transcript("pi", "sw-m", Path::new("/tmp/hp-project")), Some(first), "the remembered path wins over a fresh search");
    }
    #[test]
    fn a_moved_file_is_looked_for_again() {
        let h = home("moved", 5);
        let f = save(&h, "--tmp-hp-project--", "sw-v");
        assert_eq!(transcript("pi", "sw-v", Path::new("/tmp/hp-project")), Some(f.clone()));
        let to = save(&h, "--tmp-new--", "sw-v");
        fs::remove_file(&f).unwrap();
        assert_eq!(transcript("pi", "sw-v", Path::new("/tmp/hp-project")), Some(to));
    }
    #[test]
    fn a_linked_folder_with_a_trailing_slash_is_found_directly() {
        let h = home("link", 0);
        let target = h.root.join("real-project");
        fs::create_dir_all(&target).unwrap();
        let link = h.root.join("linked");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let f = save(&h, &pi_dir(&real(&target)), "sw-l");
        let given = PathBuf::from(format!("{}/", link.display()));
        assert_eq!(locate("pi", "sw-l", &given), Some(f));
    }
    #[test]
    fn the_memo_stays_bounded() {
        let h = home("cap", 0);
        save(&h, "--tmp-p--", "sw-c");
        for i in 0..CAP + 10 { FOUND.lock().unwrap().insert(("pi".into(), format!("x{i}")), (Some(PathBuf::from("/x")), Instant::now())); }
        assert!(transcript("pi", "sw-c", Path::new("/tmp/p")).is_some());
        assert!(FOUND.lock().unwrap().len() <= CAP);
    }
    #[test]
    fn newest_first_from_its_own_folders_only() {
        let h = home("saved", 3);
        let at = |f: &PathBuf, secs: u64| { fs::File::options().write(true).open(f).unwrap().set_modified(std::time::UNIX_EPOCH + Duration::from_secs(secs)).unwrap(); };
        let old = save(&h, "--tmp-hp-project--", "sw-old");
        let moved = save(&h, "--tmp-hp-before--", "sw-was");
        let new = save(&h, "--tmp-hp-project--", "sw-new");
        at(&old, 1_000_000); at(&moved, 2_000_000); at(&new, 3_000_000);
        let got = pi_saved(Path::new("/tmp/hp-project/"), Some(Path::new("/tmp/hp-before")));
        assert_eq!(got, vec![new, moved, old], "newest first, other folders left out");
    }
    #[test]
    fn every_folder_when_its_own_is_not_there_yet() {
        let h = home("saved-none", 2);
        assert_eq!(pi_saved(Path::new("/tmp/hp-nowhere"), None).len(), 2);
    }
    #[test]
    #[ignore]
    fn bench_lookup_3k_dirs() {
        let h = home("bench", 3000);
        remember_misses();
        save(&h, "--tmp-hp-project--", "sw-b");
        let (sessions, cwd, n) = (h.root.join("sessions"), Path::new("/tmp/hp-project"), 50u32);
        let t = Instant::now();
        for _ in 0..n { assert!(find_file(&sessions, &|f| f.ends_with("_sw-b.jsonl"), 1).into_iter().next().is_some()); }
        let walk = t.elapsed() / n;
        let t = Instant::now();
        for _ in 0..n { assert!(locate("pi", "sw-b", cwd).is_some()); }
        let direct = t.elapsed() / n;
        assert!(transcript("pi", "sw-b", cwd).is_some());
        let t = Instant::now();
        for _ in 0..n { assert!(transcript("pi", "sw-b", cwd).is_some()); }
        let memo = t.elapsed() / n;
        let t = Instant::now();
        for _ in 0..n { assert!(!pi_saved(cwd, None).is_empty()); }
        let saved = t.elapsed() / n;
        let t = Instant::now();
        for _ in 0..n {
            let mut v = find_file(&sessions, &|f| f.ends_with(".jsonl"), 1);
            let mtime = |p: &PathBuf| fs::metadata(p).and_then(|m| m.modified()).ok();
            v.sort_by(|a, b| mtime(b).cmp(&mtime(a)));
        }
        let old_latest = t.elapsed() / n;
        let t = Instant::now();
        for _ in 0..n { assert!(!find_file(&sessions, &|f| f.ends_with(".jsonl"), 1).is_empty()); }
        let all_once = t.elapsed() / n;
        let t = Instant::now();
        for _ in 0..n { assert!(transcript("pi", "sw-none", cwd).is_none()); }
        let miss = t.elapsed() / n;
        println!("BENCH miss_remembered={miss:?} dirs=3000 full_walk={walk:?} direct_dir={direct:?} memo_hit={memo:?} latest_before={old_latest:?} latest_all_time_read_once={all_once:?} latest_own_folder={saved:?}");
    }
}
