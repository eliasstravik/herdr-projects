//! The tests never touch the user's real board: under them the store refuses the real ~/.herdr-projects, loudly.
//! HERDR_PROJECTS_TESTING says the tests run (scripts/run-tests.sh sets it, with a scratch HERDR_PROJECTS_HOME). A
//! board folder that is in the real one then stops the run there, before anything is read or written in it: a fake
//! task in your board (`probe/12`, HP-172) came from a run that believed it had a folder of its own (docs/testing.md).
use std::path::{Path, PathBuf};

pub const TESTING: &str = "HERDR_PROJECTS_TESTING";
pub fn testing() -> bool { std::env::var_os(TESTING).is_some_and(|v| !v.is_empty()) }
/// The user's own home folder, from the system's list of users: what HOME says can be a test's.
pub fn real_home() -> Option<PathBuf> {
    let dir = unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() || (*pw).pw_dir.is_null() { return None; }
        std::ffi::CStr::from_ptr((*pw).pw_dir).to_string_lossy().into_owned()
    };
    (!dir.is_empty()).then(|| PathBuf::from(dir))
}
/// The real board's folder: what the board keeps for this user when nothing says otherwise.
pub fn real_board() -> Option<PathBuf> { real_home().map(|h| h.join(super::files::HOME_NAME)) }
/// A board folder, as it is: refused under the tests when it is in the real board's folder (a link to it too).
pub fn guard(p: PathBuf) -> PathBuf {
    if !testing() { return p; }
    let Some(real) = real_board() else { return p };
    if inside(&p, &real) {
        panic!("Under the tests the board's folder may not be the real one: {} is in {}. A test keeps its board in a temporary folder: world() in tests/common, or HERDR_PROJECTS_HOME set to a scratch folder (and HERDR_PROJECTS_DATA_DIR, HERDR_PROJECTS_CONFIG_DIR and HERDR_PROJECTS_REPORTS_DIR unset or scratch too). {TESTING} is set, by scripts/run-tests.sh.", p.display(), real.display());
    }
    p
}
fn inside(p: &Path, real: &Path) -> bool {
    let p = super::files::absolute(p);
    p.starts_with(real) || resolved(&p).starts_with(resolved(real))
}
/// The path with every link in the part of it that exists followed: the rest may not be there yet.
fn resolved(p: &Path) -> PathBuf {
    let mut rest = vec![];
    let mut at = p;
    loop {
        if let Ok(c) = std::fs::canonicalize(at) { return rest.iter().rev().fold(c, |c, x| c.join(x)); }
        match (at.parent(), at.file_name()) { (Some(up), Some(name)) => { rest.push(name.to_os_string()); at = up; } _ => return p.to_path_buf() }
    }
}
