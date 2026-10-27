//! Marking a folder as trusted in pi's own trust.json: a flat map of folder paths to true. A folder already marked
//! trusted is not written again, pi's own `false` is its answer and is left as it is, and a file that does not read is
//! never touched.
use super::adapter::write_private;
use super::homes::pi_home;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn file() -> PathBuf { pi_home().join("trust.json") }

/// Trusted in pi's trust.json, adding the folder's entry when it has none. False: its folder dialog shows.
pub fn trust(root: &Path) -> bool {
    let root = root.display().to_string();
    let file = file();
    let mut j: Map<String, Value> = match fs::read_to_string(&file) {
        Ok(t) => match serde_json::from_str::<Value>(&t) { Ok(Value::Object(o)) => o, _ => return false },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Map::new(),
        Err(_) => return false,
    };
    if let Some(v) = j.get(&root) { return v.as_bool() == Some(true); }
    j.insert(root, Value::Bool(true));
    if let Some(d) = file.parent() { let _ = fs::create_dir_all(d); }
    let tmp = PathBuf::from(format!("{}.{}.sw.tmp", file.display(), std::process::id()));
    write_private(&tmp, &serde_json::to_string_pretty(&Value::Object(j)).unwrap_or_default()).is_ok()
        && fs::rename(&tmp, &file).is_ok()
}
