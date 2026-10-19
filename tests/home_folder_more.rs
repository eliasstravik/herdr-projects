//! The one-time move, continued: the lock file in the place in use is never replaced; what is not the board's to move
//! stays (a folder you named, another copy's board, a worktree's copy); old places named by XDG are found; and, with
//! real programs, a lock replaced under a waiter and the old command running at every moment of a real move.
use crate::common;
use crate::move_home as util;
use common::{world_with, Opts};
use std::path::{Path, PathBuf};
use std::process::Command;
use sw::store::move_home::{move_home, swap_now};
use util::*;

fn hold(f: &Path) -> std::fs::File { use std::os::fd::AsRawFd; let h = std::fs::File::open(f).unwrap(); assert_eq!(unsafe { libc::flock(h.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0); h }
/// Whether another program finds the file locked.
fn taken(f: &Path) -> bool {
    use std::os::fd::AsRawFd;
    let Ok(h) = std::fs::OpenOptions::new().read(true).write(true).open(f) else { return false };
    let rc = unsafe { libc::flock(h.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 { unsafe { libc::flock(h.as_raw_fd(), libc::LOCK_UN) }; false } else { true }
}
fn lock_process(path: &Path, body: &str) -> std::process::Child {
    Command::new("python3").args(["-c", body]).arg(path).spawn().unwrap()
}

#[test]
fn the_lock_file_in_the_place_in_use_is_never_replaced_or_removed_and_the_new_places_copy_goes_only_while_nobody_holds_it() {
    let w = world_with(Opts::default());
    let both = || { let h = old_home(&w.base, None); let t = sw::clock::now(); let (old, now) = (h.data.join("locks/a2VlcGVy.lock"), h.to.join("data/locks/a2VlcGVy.lock")); put(&old, "", Some(t - 60000)); put(&now, "", Some(t)); (h, old, now) };
    // Both places have the lock file, the new place's the newer one. The old place's is the lock: whoever holds it
    // still has the one real lock afterwards, under both paths. (The copy that goes is taken while it is removed:
    // grab_now, a real flock, which a test cannot watch from outside.)
    let (h, old, now) = both();
    let mine = ino(&old);
    assert_eq!(h.run().moved.iter().map(|(_, how)| how.as_str()).collect::<Vec<_>>(), vec!["merged", "moved", "moved"]);
    assert_eq!((ino(&now), ino(&old)), (mine, mine)); assert!(!h.to.join("set-aside").exists());
    // The new place's copy is held by a running process: nothing of the data is moved now, the run stays on the old
    // place, both files are as they were, and a later run moves it.
    let (h, old, now) = both();
    let (mine, other) = (ino(&old), ino(&now));
    let held = hold(&now);
    let r = h.run();
    assert_eq!(r.moved.iter().map(|(p, _)| p.what).collect::<Vec<_>>(), vec!["settings", "reports"]); assert_eq!(r.stay.iter().map(|p| p.what).collect::<Vec<_>>(), vec!["data"]);
    assert_eq!((ino(&old), ino(&now)), (mine, other)); assert!(!is_link(&h.data));
    drop(held);
    assert_eq!(hows(&h.run()), vec![("data", "merged")]); assert!(is_link(&h.data)); assert_eq!(ino(&now), mine);
}

#[test]
fn what_is_not_the_boards_to_move_stays_a_folder_you_named_another_copys_board_old_places_named_by_xdg_are_found() {
    let w = world_with(Opts::default());
    // A board kept somewhere of its own (the tests' own, too): its folders are where you put them.
    let h = old_home(&w.base, None);
    let mine = (h.home.join("mine").display().to_string(), h.home.join("mine-too").display().to_string());
    let r = run_with(&h.home, &env(&[("HERDR_PROJECTS_DATA_DIR", &mine.0), ("HERDR_PROJECTS_CONFIG_DIR", &mine.1)]), &swap_now);
    assert_eq!(r.moved.iter().map(|(p, _)| p.what).collect::<Vec<_>>(), vec!["reports"]); assert!(!is_link(&h.data) && !is_link(&h.settings));
    // The old data folder's command names another copy of the program that is still there (the installed one, seen
    // from a worktree): this copy moves nothing, and is told so that it works on that board where it is.
    let h = old_home(&w.base, Some("/bin/sh"));
    let r = h.run();
    assert!(r.other && r.moved.is_empty() && !is_link(&h.data)); assert_eq!(names(&h.to), vec!["closing-1.md"]);
    assert_eq!(r.stay.iter().map(|p| (p.key, p.from.clone())).collect::<Vec<_>>(), vec![("HERDR_PROJECTS_DATA_DIR", h.data.clone()), ("HERDR_PROJECTS_CONFIG_DIR", h.settings.clone()), ("HERDR_PROJECTS_REPORTS_DIR", h.reports.clone())]);
    // A copy that is a git worktree (a task's agent at work on this code, its reviewer trying it) never moves the board's files.
    let h = old_home(&w.base, None);
    let r = move_home(&h.home, &env(&[]), &bin_of(&h.home), Some(true), 3000, &swap_now);
    assert!(r.other && !is_link(&h.data)); assert_eq!(h.run().moved.len(), 3);
    // A copy that is gone (the plugin was moved) holds nothing back.
    let h = old_home(&w.base, Some("/no/such/copy/bin/herdr-projects"));
    assert_eq!(h.run().moved.len(), 3);
    // The old version followed XDG_*: that is where its folders are looked for.
    let h = old_home(&w.base, None);
    let x = h.home.join("xdg");
    put(&x.join("state/herdr-superwork/tasks/a/1/task.json"), "{}", None); put(&x.join("config/herdr-superwork/config.json"), r#"{"max":3}"#, None);
    let r = run_with(&h.home, &env(&[("XDG_STATE_HOME", x.join("state").to_str().unwrap()), ("XDG_CONFIG_HOME", x.join("config").to_str().unwrap())]), &swap_now);
    assert_eq!(r.moved.iter().map(|(p, _)| p.from.clone()).collect::<Vec<_>>(), vec![x.join("state/herdr-superwork"), x.join("config/herdr-superwork"), h.reports.clone()]);
    assert_eq!(read(&h.to.join("settings/config.json")), r#"{"max":3}"#); assert!(is_link(&x.join("state/herdr-superwork")));
    assert!(!is_link(&h.data)); // the folders in the usual old places were not the ones in use: left alone
}

#[test]
fn real_programs_a_lock_file_that_was_replaced_between_opening_it_and_locking_it_is_not_taken_for_the_lock() {
    let w = world_with(Opts::default());
    let file = sw::store::files::data_dir().join("locks").join(format!("{}.lock", sw::store::files::base64url(b"x")));
    put(&file, "", None);
    // Someone holds the file; while this process waits for it (the file open, not yet locked), another file takes its
    // place at the path, and then the holder lets go of the old one.
    let body = "import fcntl, os, sys, time; path=sys.argv[1]; held=open(path, 'r+'); fcntl.flock(held, fcntl.LOCK_EX); time.sleep(0.25); tmp=path+'.new'; open(tmp, 'w').close(); os.replace(tmp, path); time.sleep(0.1)";
    let mut holder = lock_process(&file, body);
    for _ in 0..200 { if taken(&file) { break; } std::thread::sleep(std::time::Duration::from_millis(5)); }
    let was = ino(&file);
    let (seen, held) = sw::store::files::with_lock_wait("x", 5000, || Ok((ino(&file), taken(&file)))).unwrap();
    assert_ne!(seen, was); assert!(held, "what it holds is the file at the path, so nobody else can take that");
    assert!(!taken(&file)); // and let go again
    let _ = holder.kill(); let _ = holder.wait();
    drop(w);
}

#[test]
fn real_programs_the_old_command_runs_at_every_moment_of_a_real_move_and_a_lock_held_through_it_stays_held() {
    let w = world_with(Opts::default());
    let h = old_home(&w.base, None);
    let launcher = h.data.join("bin").join(OLD_NAME);
    put(&launcher, "#!/bin/sh\necho ran\n", None);
    use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755)).unwrap();
    let lock = h.data.join("locks/dGFzaw.lock");
    let body = "import fcntl, sys, time; held=open(sys.argv[1], 'a'); fcntl.flock(held, fcntl.LOCK_EX); time.sleep(30)";
    let mut holder = lock_process(&lock, body);
    for _ in 0..200 { if taken(&lock) { break; } std::thread::sleep(std::time::Duration::from_millis(10)); }
    // At each of the two steps in which paths change places, before and after it, the old command is started.
    let ran = std::cell::RefCell::new(vec![]);
    let old = || ran.borrow_mut().push(String::from_utf8_lossy(&Command::new(&launcher).output().map(|o| o.stdout).unwrap_or_default()).into_owned());
    let own = h.home.join("own").display().to_string();
    let r = run_with(&h.home, &env(&[("HERDR_PROJECTS_CONFIG_DIR", &own), ("HERDR_PROJECTS_REPORTS_DIR", &own)]), &|a: &Path, b: &Path| { old(); let ok = swap_now(a, b); old(); ok });
    assert_eq!(hows(&r), vec![("data", "moved")]);
    assert_eq!(*ran.borrow(), vec!["ran\n"; 4]);
    assert!(is_link(&h.data) && is_dir(&h.to.join("data")));
    assert!(taken(&lock) && taken(&PathBuf::from(&h.to).join("data/locks/dGFzaw.lock"))); // still the holder's, by either path
    let _ = holder.kill(); let _ = holder.wait();
}
