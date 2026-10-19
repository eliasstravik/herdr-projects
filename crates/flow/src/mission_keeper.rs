//! Keep Mission Control's scheduler on its own source build when an older installed release is still ticking.
use crate::json::*;
use crate::store::files::{bin, data_dir, marker_alive, read_json};

pub fn ensure() {
    if crate::runner::inline() { return; }
    if let Some(beat) = read_json(&data_dir().join("keeper.json")) {
        let pid = beat.i("pid");
        if pid > 1 && marker_alive(&beat) {
            if let Ok(output) = std::process::Command::new("ps").args(["-p", &pid.to_string(), "-o", "args="]).output() {
                let args = String::from_utf8_lossy(&output.stdout);
                if let Some(program) = args.trim().strip_suffix(" keeper") {
                    let old = std::path::Path::new(program);
                    let owned = old.file_name().map(|n| n.to_string_lossy().starts_with("herdr-projects")).unwrap_or(false);
                    let real = |p: &std::path::Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
                    if owned && real(old) != real(&bin()) {
                        unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                        return;
                    }
                }
            }
        }
    }
    crate::keeper::ensure_keeper();
}
