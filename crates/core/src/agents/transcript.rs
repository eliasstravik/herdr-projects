//! Where a program saved a conversation, found once and remembered: the keeper asks for every working task on every
//! tick, and a search of a large sessions folder each time costs most of a core.
use super::adapter::{dashed, real};
use super::homes::*;
use super::read_turn::find_file;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

/// The folder pi saves a directory's conversations in: `/a/b` becomes `--a-b--`.
fn pi_dir(cwd: &Path) -> String { format!("--{}--", cwd.display().to_string().trim_start_matches('/').replace('/', "-")) }
static FOUND: LazyLock<Mutex<HashMap<(String, String), PathBuf>>> = LazyLock::new(Default::default);
/// Where a conversation is saved. A conversation stays where it was first found, so the answer is remembered and
/// looked for again only once the file is gone: the keeper asks for every working task on every tick.
pub fn transcript(kind: &str, session: &str, cwd: &Path) -> Option<PathBuf> {
    let key = (kind.to_string(), session.to_string());
    let known = FOUND.lock().unwrap().get(&key).cloned();
    if let Some(f) = known {
        if f.exists() { return Some(f); }
        FOUND.lock().unwrap().remove(&key);
    }
    let found = locate(kind, session, cwd);
    if let Some(f) = &found { FOUND.lock().unwrap().insert(key, f.clone()); }
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
            for dir in [cwd.to_path_buf(), real(cwd)] {
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
    fn tree(dirs: usize) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("hp-pi-{}-{dirs}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let sessions = root.join("sessions");
        for i in 0..dirs { let d = sessions.join(format!("--tmp-other-{i}--")); fs::create_dir_all(&d).unwrap(); fs::write(d.join(format!("2026_x{i}.jsonl")), "").unwrap(); }
        let cwd = PathBuf::from("/tmp/hp-project");
        let mine = sessions.join(pi_dir(&cwd));
        fs::create_dir_all(&mine).unwrap();
        fs::write(mine.join("2026_sw-target.jsonl"), "").unwrap();
        (root, cwd)
    }
    #[test]
    fn pi_dir_matches_pi_layout() { assert_eq!(pi_dir(Path::new("/Users/luke")), "--Users-luke--"); }
    #[test]
    fn finds_pi_conversation_remembers_it_and_forgets_a_removed_file() {
        let (root, cwd) = tree(20);
        unsafe { std::env::set_var("PI_CODING_AGENT_DIR", &root); }
        let f = transcript("pi", "sw-target", &cwd).expect("found in its own folder");
        assert!(f.ends_with("2026_sw-target.jsonl"));
        let moved = root.join("sessions/--tmp-elsewhere--");
        fs::create_dir_all(&moved).unwrap();
        fs::rename(&f, moved.join("2026_sw-target.jsonl")).unwrap();
        assert_eq!(transcript("pi", "sw-target", &cwd), Some(moved.join("2026_sw-target.jsonl")), "a missing file is looked for again");
        assert_eq!(transcript("pi", "sw-absent", &cwd), None);
        let _ = fs::remove_dir_all(&root);
    }
    #[test]
    #[ignore]
    fn bench_lookup_3k_dirs() {
        let (root, cwd) = tree(3000);
        let sessions = root.join("sessions");
        let n = 50;
        let t = std::time::Instant::now();
        for _ in 0..n { assert!(find_file(&sessions, &|f| f.ends_with("_sw-target.jsonl"), 1).into_iter().next().is_some()); }
        let walk = t.elapsed() / n;
        unsafe { std::env::set_var("PI_CODING_AGENT_DIR", &root); }
        FOUND.lock().unwrap().clear();
        let t = std::time::Instant::now();
        assert!(locate("pi", "sw-target", &cwd).is_some());
        let direct = t.elapsed();
        let t = std::time::Instant::now();
        for _ in 0..n { assert!(transcript("pi", "sw-target", &cwd).is_some()); }
        let memo = t.elapsed() / n;
        println!("BENCH dirs=3000 full_walk={walk:?} direct_dir={direct:?} memo_hit={memo:?}");
        let _ = fs::remove_dir_all(&root);
    }
}
