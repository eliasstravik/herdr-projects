mod actions;
mod adopt;
mod agents;
mod cli;
mod coordinator;
mod doctor;
mod herdr;
mod inbox;
mod lifecycle;
mod names;
mod overview;
mod paths;
mod pr;
mod progress;
mod project;
mod remote;
mod routine;
mod runner;
#[cfg(test)]
mod scenarios;
mod setup;
mod sidebar;
mod steps;
mod thread;
mod threads;
mod ticker;

/// Crate version plus a build identifier (short git hash and build time), so a
/// rebuilt binary always differs from the one a running ticker was started from.
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("HP_BUILD_ID"));

/// A herdr server that was not started from a login shell hands its plugins a
/// minimal `PATH`, so `gh`, `rsync` or the agent CLI may be missing for the
/// ticker although they work in the user's terminal. The usual install folders
/// are appended (never prepended: what the user's `PATH` resolves still wins).
fn extend_path() {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs: Vec<std::path::PathBuf> = std::env::split_paths(&current).collect();
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let mut extra: Vec<std::path::PathBuf> = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"].iter().map(Into::into).collect();
    if let Some(home) = home {
        extra.push(home.join(".local/bin"));
        extra.push(home.join(".cargo/bin"));
    }
    for dir in extra {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    if let Ok(joined) = std::env::join_paths(dirs) {
        // SAFETY: first thing in `main`, before any thread exists.
        unsafe { std::env::set_var("PATH", joined) };
    }
}

fn main() {
    extend_path();
    if let Err(error) = cli::run() {
        eprintln!("herdr-projects: {error:#}");
        std::process::exit(1);
    }
}
