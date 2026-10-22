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
/// How many answers are kept; a long-running keeper sees many conversations come and go.
const CAP: usize = 1024;
static FOUND: LazyLock<Mutex<HashMap<(String, String), (PathBuf, Instant)>>> = LazyLock::new(Default::default);

/// `a/./b/../c/` becomes `a/c`, without following links, as pi's path.resolve does.
fn lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c { Component::CurDir => {}, Component::ParentDir => { out.pop(); }, c => out.push(c) }
    }
    out
}
/// The folder pi saves a directory's conversations in: one leading separator dropped, then `/`, `\\` and `:` become `-`.
fn pi_dir(cwd: &Path) -> String {
    let s = cwd.display().to_string();
    let s = s.strip_prefix(['/', '\\']).unwrap_or(&s);
    format!("--{}--", s.replace(['/', '\\', ':'], "-"))
}
/// Where a conversation is saved. The answer is remembered for a while and dropped early once the file is gone.
pub fn transcript(kind: &str, session: &str, cwd: &Path) -> Option<PathBuf> {
    let key = (kind.to_string(), session.to_string());
    let known = FOUND.lock().unwrap().get(&key).cloned();
    if let Some((f, at)) = known {
        if at.elapsed() < FRESH && f.is_file() { return Some(f); }
        FOUND.lock().unwrap().remove(&key);
    }
    let found = locate(kind, session, cwd);
    if let Some(f) = &found {
        let mut map = FOUND.lock().unwrap();
        if map.len() >= CAP { map.retain(|_, (_, at)| at.elapsed() < FRESH); }
        if map.len() >= CAP { map.clear(); }
        map.insert(key, (f.clone(), Instant::now()));
    }
    found
}
fn locate(kind: &str, session: &str, cwd: &Path) -> Option<PathBuf> {
    match kind {
        "claude" => {
            let direct = claude_home().join("projects").join(dashed(&real(cwd))).join(format!("{session}.jsonl"));
            if direct.exists() { return Some(direct); }
            let want = format!("{session}.jsonl");
            find_file(&claude_home().join("projects"), &|n| n == want, 1).into_iter().next()
        }
        "codex" => { let want = format!("{session}.jsonl"); find_file(&codex_home().join("sessions"), &|n| n.ends_with(&want), 4).into_iter().next() }
        "pi" => {
            let want = format!("_{session}.jsonl");
            let root = pi_home().join("sessions");
            let given = lexical(cwd);
            for dir in [given.clone(), real(&given)] {
                if let Some(f) = find_file(&root.join(pi_dir(&dir)), &|n| n.ends_with(&want), 0).into_iter().next() { return Some(f); }
            }
            find_file(&root, &|n| n.ends_with(&want), 1).into_iter().next()
        }
        _ => None,
    }
}

#[cfg(test)]
mod transcript_tests {
    use super::*;
    use std::fs;
    /// Tests share the process environment and the memo, so they take turns.
    static TURN: Mutex<()> = Mutex::new(());
    struct Home { root: PathBuf, was: Option<std::ffi::OsString>, _turn: std::sync::MutexGuard<'static, ()> }
    impl Drop for Home {
        fn drop(&mut self) {
            unsafe { match &self.was { Some(v) => std::env::set_var("PI_CODING_AGENT_DIR", v), None => std::env::remove_var("PI_CODING_AGENT_DIR") } }
            FOUND.lock().unwrap().clear();
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
        Home { root, was, _turn: turn }
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
        for i in 0..CAP + 10 { FOUND.lock().unwrap().insert(("pi".into(), format!("x{i}")), (PathBuf::from("/x"), Instant::now())); }
        assert!(transcript("pi", "sw-c", Path::new("/tmp/p")).is_some());
        assert!(FOUND.lock().unwrap().len() <= CAP);
    }
    #[test]
    #[ignore]
    fn bench_lookup_3k_dirs() {
        let h = home("bench", 3000);
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
        println!("BENCH dirs=3000 full_walk={walk:?} direct_dir={direct:?} memo_hit={memo:?}");
    }
}
