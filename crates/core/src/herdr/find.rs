//! Where the herdr program is when this process's PATH does not have it: a plain `ssh host cmd` gets no login shell,
//! so `~/.local/bin` and the like are often missing. The usual install spots are looked in, then a login shell's PATH,
//! once per process. Used by cli.rs (`bin`).
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Where installers put herdr, under the home folder or not: looked in after PATH, in this order.
pub const SPOTS: [&str; 5] = ["~/.local/bin", "~/.cargo/bin", "~/bin", "/usr/local/bin", "/opt/homebrew/bin"];
/// The usual spots, `~` being `home`.
pub fn spots(home: &Path) -> Vec<PathBuf> { SPOTS.iter().map(|s| s.strip_prefix("~/").map(|r| home.join(r)).unwrap_or_else(|| PathBuf::from(s))).collect() }

fn runs(p: &Path) -> bool { use std::os::unix::fs::PermissionsExt; std::fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false) }
/// The first of `dirs` (a PATH, `:` between) that has a program `name` that runs.
pub fn in_dirs(name: &str, dirs: &str) -> Option<PathBuf> { dirs.split(':').filter(|d| !d.is_empty()).map(|d| Path::new(d).join(name)).find(|p| runs(p)) }
/// Where `name` is: None when `path` has it (the name alone runs it), else the first of `spots` that has it, else the
/// first dir of `login_path` (asked only then) that has it; None too when nothing has it.
pub fn elsewhere(name: &str, path: &str, spots: &[PathBuf], login_path: impl FnOnce() -> String) -> Option<PathBuf> {
    if in_dirs(name, path).is_some() { return None; }
    spots.iter().map(|d| d.join(name)).find(|p| runs(p)).or_else(|| in_dirs(name, &login_path()))
}
/// herdr's full path when this process's PATH lacks it and it is found elsewhere; asked once per process.
pub fn herdr_elsewhere() -> Option<String> {
    static FOUND: OnceLock<Option<String>> = OnceLock::new();
    FOUND.get_or_init(|| {
        let path = std::env::var("PATH").unwrap_or_default();
        elsewhere("herdr", &path, &spots(&crate::store::files::user_home()), || crate::login_env::login_shell().map(|o| crate::login_env::path_of(&o)).unwrap_or_default()).map(|p| p.display().to_string())
    }).clone()
}
