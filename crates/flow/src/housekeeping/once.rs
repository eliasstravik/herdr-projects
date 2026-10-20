//! At most one housekeeping pass per herdr session at a time (#257). Every runner ends with a forced pass, and a pass
//! takes minutes when herdr answers slowly: passes that overlapped each started more runners, each ending in another
//! pass, until the user's process limit was reached. A pass that finds one under way leaves it a mark (when it was
//! forced) and returns: the one under way runs once more at its end, as forced as the strongest mark, so no PR poll
//! asked for is lost.
use super::Force;
use crate::store::{files::hold_lock, marks::stamp_file};
use std::path::PathBuf;

fn owed(key: &str, f: Force) -> PathBuf { stamp_file(&format!("owed-{}-{key}", if f == Force::Pr { "pr" } else { "yes" })) }
fn owe(key: &str, f: Force) {
    if f == Force::No { return; }
    let p = owed(key, f);
    if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
    let _ = std::fs::write(p, "");
}
/// The marks left so far, taken: each is removed by exactly one taker.
fn take(key: &str) -> Option<Force> {
    let pr = std::fs::remove_file(owed(key, Force::Pr)).is_ok();
    let yes = std::fs::remove_file(owed(key, Force::Yes)).is_ok();
    if pr { Some(Force::Pr) } else if yes { Some(Force::Yes) } else { None }
}
fn any(key: &str) -> bool { owed(key, Force::Pr).exists() || owed(key, Force::Yes).exists() }
fn stronger(a: Force, b: Force) -> Force { if a == Force::Pr || b == Force::Pr { Force::Pr } else if a == Force::Yes || b == Force::Yes { Force::Yes } else { Force::No } }

/// Run `pass` unless a pass of this session is under way, then again for each mark left meanwhile.
pub fn once(key: &str, force: Force, mut pass: impl FnMut(Force)) {
    owe(key, force); // first, so the pass under way sees it even if it lets go of the lock right now
    let mut mine = Some(force);
    loop {
        let Ok(held) = hold_lock(&format!("housekeeping:{key}"), 0) else { return };
        loop {
            let f = match (mine.take(), take(key)) { (Some(a), Some(b)) => stronger(a, b), (Some(f), None) | (None, Some(f)) => f, (None, None) => break };
            pass(f);
        }
        drop(held);
        // A mark left after the last look and before the lock was let go: its pass found the lock still taken.
        if !any(key) { return; }
    }
}
