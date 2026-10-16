//! The program itself, for `doctor`: the version that runs and how it was installed, whether the launcher in the data
//! folder (what the agents and hooks run) names the plugin's own program, and whether a shell finds it on the PATH.
use super::Found;
use crate::install::{asset_name, built_from_source, newer, path_problem, platform};
use crate::store::marks::tilde;
use std::path::{Path, PathBuf};

/// What is looked at. `plugin_root`: the folder herdr runs the plugin from, when herdr lists it.
pub struct Here { pub exe: PathBuf, pub version: String, pub launcher: PathBuf, pub launcher_text: Option<String>, pub plugin_root: Option<PathBuf>, pub path: String }
pub fn here(launcher: PathBuf, launcher_text: Option<String>, plugin_root: Option<PathBuf>) -> Here {
    let exe = crate::store::files::bin();
    Here { exe: std::fs::canonicalize(&exe).unwrap_or(exe), version: crate::store::files::VERSION.into(), launcher, launcher_text, plugin_root, path: std::env::var("PATH").unwrap_or_default() }
}
fn t(p: &Path) -> String { tilde(&p.display().to_string()) }
fn real(p: &Path) -> PathBuf { std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()) }
/// The program a launcher runs: its `exec '<program>' "$@"` line (flow/send.rs writes it).
pub fn launcher_target(text: &str) -> Option<PathBuf> {
    let q = text.lines().find_map(|l| l.strip_prefix("exec "))?.strip_suffix(" \"$@\"")?;
    let inner = q.strip_prefix('\'')?.strip_suffix('\'')?;
    Some(PathBuf::from(inner.replace("'\\''", "'")))
}
/// The programs the plugin runs (bin/run.sh): the one built in its folder, or the one downloaded for this platform.
pub fn plugin_programs(root: &Path) -> Vec<PathBuf> {
    let mut v = vec![root.join("target/release/herdr-projects")];
    v.extend(platform().map(|p| root.join("bin").join(asset_name(p))));
    v
}
/// The version a plugin folder is (its Cargo.toml).
fn plugin_version(root: &Path) -> Option<String> {
    std::fs::read_to_string(root.join("Cargo.toml")).ok()?.lines().find_map(|l| l.strip_prefix("version = \"").and_then(|r| r.split('"').next()).map(String::from))
}
fn how(h: &Here) -> &'static str {
    let downloaded = platform().map(|p| h.exe.file_name().map(|n| n == asset_name(p).as_str()).unwrap_or(false)).unwrap_or(false);
    if built_from_source(&h.exe) { "built from source" } else if downloaded { "downloaded" } else { "installed by hand" }
}
fn is_program(p: &Path) -> bool { use std::os::unix::fs::PermissionsExt; std::fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false) }

pub fn check(h: &Here) -> Vec<Found> {
    let mut out = vec![Found::ok(format!("herdr-projects {}, {}: {}", h.version, how(h), t(&h.exe)))];
    let root = h.plugin_root.as_deref();
    // The plugin's own folder is a newer version than the program it runs.
    if let (Some(r), Some(pv)) = (root, root.and_then(plugin_version)) {
        if h.exe.starts_with(real(r)) && newer(&pv, &h.version) {
            let fix = if built_from_source(&h.exe) { format!("Build it again: cd {} && cargo build --release", t(r)) } else { format!("Run: herdr-projects update {pv}") };
            out.push(Found::bad(format!("This program is {}, older than the plugin's {pv}", h.version), fix));
        }
    }
    let fix = match root { Some(r) => format!("Run: sh {}/bin/run.sh install  (or open the board once from Herdr)", t(r)), None => "Open the board once from Herdr, or run: herdr-projects install".into() };
    let l = t(&h.launcher);
    let target = match &h.launcher_text {
        None => { out.push(Found::bad(format!("The launcher {l} is missing: agents and hooks run herdr-projects through it"), fix)); return out; }
        Some(text) => launcher_target(text),
    };
    if h.launcher.exists() && !is_program(&h.launcher) { out.push(Found::bad(format!("The launcher {l} cannot be run: it is not executable"), fix)); return out; }
    let Some(target) = target else { out.push(Found::bad(format!("The launcher {l} names no program"), fix)); return out; };
    if !is_program(&target) { out.push(Found::bad(format!("The launcher {l} runs {}, which is not there", t(&target)), fix)); return out; }
    if let Some(r) = root {
        if !plugin_programs(r).iter().any(|p| real(p) == real(&target)) {
            out.push(Found::bad(format!("The launcher {l} runs {}, not the plugin's program in {}", t(&target), t(r)), fix));
            return out;
        }
    }
    match path_problem(&h.launcher, &target, &h.path) {
        None => out.push(Found::ok(format!("The launcher {l} runs {}, and a shell finds it on the PATH", t(&target)))),
        Some((what, fix)) => { out.push(Found::ok(format!("The launcher {l} runs {}", t(&target)))); out.push(Found::bad(what, fix)); }
    }
    out
}
