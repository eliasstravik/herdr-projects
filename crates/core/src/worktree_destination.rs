//! Worktree destinations: physical parent folders, exclusive directory reservations and validation of Herdr's answer.
use crate::err::Res;
use crate::json::*;
use crate::store::repos::{git_out, primary_root};
use crate::store::settings::lookup::setting_text;
use crate::store::tasks::{update_wait, Change, Task};
use std::fs;
use std::path::{Path, PathBuf};

/// Resolve a configured parent without creating it, so a start can check its volume before writing anything.
pub fn configured(root: &Path, t: &Task) -> Res<Option<PathBuf>> {
    let value = setting_text("workspace.destination", Some(t));
    if value.is_empty() || value == "none" {
        return Ok(None);
    }
    let primary = fs::canonicalize(primary_root(root))?;
    Ok(Some(primary.join(value)))
}

/// Resolve at creation time, against the physical primary checkout (not a linked worktree or a symlink to it).
pub fn parent(root: &Path, t: &Task) -> Res<Option<PathBuf>> {
    let Some(dest) = configured(root, t)? else {
        return Ok(None);
    };
    fs::create_dir_all(&dest)?;
    Ok(Some(fs::canonicalize(dest)?))
}

/// Claim a name atomically in a hidden reservation folder; leave the checkout path absent for Herdr and Git.
/// Registered worktrees whose folders have disappeared are occupied too; no pruning or reuse is done here.
pub fn reserve(root: &Path, parent: &Path, branch: &str) -> Res<PathBuf> {
    fs::create_dir_all(parent)?;
    let registered = git_out(root, &["worktree", "list", "--porcelain", "-z"])?;
    let paths: Vec<&Path> = registered
        .split('\0')
        .filter_map(|s| s.strip_prefix("worktree "))
        .map(Path::new)
        .collect();
    let reservations = parent.join(".herdr-projects-reservations");
    fs::create_dir_all(&reservations)?;
    let name: String = branch
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .take(120)
        .collect();
    let name = if name.is_empty() || name == "." || name == ".." {
        "task"
    } else {
        &name
    };
    for n in 1.. {
        let dir = parent.join(if n == 1 {
            name.to_string()
        } else {
            format!("{name}-{n}")
        });
        if paths.iter().any(|p| *p == dir) || occupied(&dir)? {
            continue;
        }
        match fs::create_dir(reservations.join(dir.file_name().unwrap())) {
            Ok(()) if !occupied(&dir)? => return Ok(dir),
            Ok(()) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!()
}

fn occupied(dir: &Path) -> Res<bool> {
    match fs::symlink_metadata(dir) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

pub fn returned(res: &Value) -> String {
    if res.at("worktree").b("path") {
        res.at("worktree").t("path")
    } else {
        res.at("workspace").at("worktree").t("checkout_path")
    }
}

/// Save the answer before checking it. An unverified checkout is diagnostic data, never an owned task worktree.
pub fn record(
    root: &Path,
    t: &Task,
    requested: Option<&Path>,
    branch: &str,
    res: &Value,
) -> Res<()> {
    let Some(expected) = requested else {
        return Ok(());
    };
    let actual = returned(res);
    update_wait(t.s("id"), 10000, |t| {
        t.put(
            "worktreeLocation",
            json!({ "requested": expected.display().to_string(), "returned": actual }),
        );
        Ok(Change::Write)
    })?;
    validate(root, expected, &actual, branch)
}

/// Exact physical path, same repository, own branch, and a linked checkout: a parent-folder match alone is unsafe.
pub fn validate(root: &Path, expected: &Path, actual: &str, branch: &str) -> Res<()> {
    let dir = Path::new(actual);
    let valid = dir.is_absolute()
        && fs::canonicalize(dir).ok().as_deref() == Some(expected)
        && fs::symlink_metadata(dir.join(".git"))
            .map(|m| m.is_file())
            .unwrap_or(false)
        && fs::canonicalize(primary_root(dir)).ok() == fs::canonicalize(primary_root(root)).ok()
        && git_out(dir, &["symbolic-ref", "--short", "HEAD"])
            .ok()
            .as_deref()
            == Some(branch);
    if !valid {
        crate::fail!("Worktree destination mismatch: requested {}, Herdr returned {:?}. No setup or worker was started in it. Inspect the returned checkout and Herdr's --path support, correct workspace.destination if needed, then delegate again. Existing worktrees have been left in place.", expected.display(), actual);
    }
    Ok(())
}
