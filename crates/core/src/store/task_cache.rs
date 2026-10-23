//! The task files as this process last read them, so that a look at every task (the board's each second, the keeper's,
//! the web board's) parses only the files that changed since: a file is known by its inode, size and times, which every
//! write changes (ours rename a new file over it). A file changed lately is also compared by its bytes, as two writes in
//! the same tick of the file system's clock can leave its times as they were. The tasks are shared, not copied.
use super::files::*;
use super::tasks::{as_now, Task};
use crate::json::*;
use std::cell::Cell;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long after its last change a file's times are trusted alone: well past any file system's clock tick.
const SETTLE: Duration = Duration::from_secs(2);
/// What the file system says of a file: a write changes at least one of these, but in the tick it was read.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp { ino: u64, size: u64, mtime: (i64, i64), ctime: (i64, i64) }
/// A file as read: its task in the names of this version (None: not a task), and its bytes until it settled.
struct Seen { stamp: Stamp, task: Option<Arc<Task>>, bytes: Option<Vec<u8>> }
/// The tasks folder they are in, and each task by its id.
static SEEN: Mutex<Option<(PathBuf, HashMap<String, Seen>)>> = Mutex::new(None);
thread_local! { static PARSED: Cell<usize> = const { Cell::new(0) }; }
/// How many task files this thread's looks at every task have parsed: what tests count.
pub fn parsed() -> usize { PARSED.with(|p| p.get()) }

fn file_of(id: &str) -> PathBuf { dir_of(id).join("task.json") }
fn stamp(m: &std::fs::Metadata) -> Stamp { Stamp { ino: m.ino(), size: m.size(), mtime: (m.mtime(), m.mtime_nsec()), ctime: (m.ctime(), m.ctime_nsec()) } }
/// Changed before `now` by more than a tick could hide: a later write shows in its times.
fn settled(s: &Stamp, now: Duration) -> bool {
    let at = |(sec, ns): (i64, i64)| Duration::new(sec.max(0) as u64, ns.clamp(0, 999_999_999) as u32);
    at(s.mtime).max(at(s.ctime)) + SETTLE < now
}
/// Converted once, when parsed: what converts a task reads nothing but the task (but for a task in the shape of the
/// settings before their move, which reads your old file too; the move writes every such task before it writes that).
fn parse(bytes: &[u8]) -> Option<Arc<Task>> { match serde_json::from_slice(bytes) { Ok(Value::Object(o)) if o.b("id") => Some(Arc::new(as_now(o))), _ => None } }
/// The tasks of `ids` as their files hold them now, those `keep` keeps. A file gone is gone from the cache too.
pub fn read(ids: &[String], keep: &dyn Fn(&Task) -> bool) -> Vec<Arc<Task>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let dir = tasks_dir();
    let mut guard = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    let mut seen = match guard.take() { Some((d, s)) if d == dir => s, _ => HashMap::new() };
    let mut next: HashMap<String, Seen> = HashMap::with_capacity(ids.len());
    // Stat first, read after: a write between the two leaves a stamp older than what was read, and is read again.
    let mut changed: Vec<(String, Stamp, Option<Vec<u8>>)> = vec![];
    for id in ids {
        let Ok(m) = std::fs::metadata(file_of(id)) else { continue };
        let st = stamp(&m);
        match seen.remove(id) {
            Some(mut s) if s.stamp == st => {
                if let Some(old) = &s.bytes {
                    let now_bytes = std::fs::read(file_of(id)).unwrap_or_default();
                    if *old != now_bytes { changed.push((id.clone(), st, Some(now_bytes))); continue; }
                    if settled(&st, now) { s.bytes = None; }
                }
                next.insert(id.clone(), s);
            }
            _ => changed.push((id.clone(), st, None)),
        }
    }
    let read_one = |(id, st, bytes): &(String, Stamp, Option<Vec<u8>>)| -> (String, Seen) {
        let bytes = bytes.clone().or_else(|| std::fs::read(file_of(id)).ok()).unwrap_or_default();
        let task = parse(&bytes);
        (id.clone(), Seen { stamp: *st, task, bytes: if settled(st, now) { None } else { Some(bytes) } })
    };
    // Many at once (the first look of a board with a long history) on several threads.
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 8);
    if changed.len() < 32 || n == 1 { next.extend(changed.iter().map(read_one)); }
    else {
        let chunk = changed.len().div_ceil(n);
        let read_one = &read_one;
        std::thread::scope(|sc| {
            let hs: Vec<_> = changed.chunks(chunk).map(|c| sc.spawn(move || c.iter().map(read_one).collect::<Vec<_>>())).collect();
            for h in hs { next.extend(h.join().unwrap_or_default()); }
        });
    }
    PARSED.with(|p| p.set(p.get() + changed.len()));
    let out = ids.iter().filter_map(|id| next.get(id)?.task.as_ref()).filter(|t| keep(t)).cloned().collect();
    *guard = Some((dir, next));
    out
}
