//! Files that go with words to another machine: a file of this machine that the words name (`@<path>`, as an attached
//! file is named, or a path dropped in the terminal) is sent over the board's own SSH link into the folder its reader
//! there reads (`herdr-projects remote keep`, the file on its stdin), and the words name it where it landed. A file
//! that could not be sent stops the words: never a path the reader cannot open.
use super::Machine;

/// Who on the other machine the file is for: a task's agent (its id there), or a coordinator (`to` as the dispatch
/// names it: "" its top-level one, "machine", "project:<repo>").
#[derive(Clone, Debug, PartialEq)]
pub enum For { Task(String), Coordinator(String) }

/// One file sent to `m` for `to`, kept there as `keep` keeps one here (store/attached.rs): its path there.
pub fn send_bytes(m: &Machine, name: &str, kind: &str, bytes: &[u8], to: &For) -> Result<String, String> {
    let _ = (m, name, kind, bytes, to);
    Err("not yet".into())
}
/// The words, with every file of this machine they name sent to `m` for `to` and named where it landed there.
/// Err: why one could not be sent (nothing should then be sent).
pub fn carry(m: &Machine, text: &str, to: &For) -> Result<String, String> {
    let _ = (m, to);
    Ok(text.to_string())
}
