//! The `herdr-projects` command on `PATH`: a link in `~/.local/bin` (or
//! `$XDG_BIN_HOME`) to the plugin's binary, made at every plugin start and by
//! `doctor --fix`, so a reinstall into another folder never leaves it broken.
//! Only a missing entry, or a link that is dangling or points into Herdr's
//! plugin folder, is ever (re)written; a file or anyone else's link is left alone.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::paths::Env;

pub const NAME: &str = "herdr-projects";

/// The folder the link goes in: `$XDG_BIN_HOME`, else `~/.local/bin`.
pub fn bin_dir(env: &Env) -> PathBuf {
    env.var("XDG_BIN_HOME").map(PathBuf::from).unwrap_or_else(|| env.home.join(".local/bin"))
}

/// `herdr-projects` in the bin folder (`herdr-projects.cmd` on Windows).
pub fn link_path(env: &Env) -> PathBuf {
    bin_dir(env).join(crate::platform::command_link_name(NAME))
}

/// Whether `binary` is an installed build (`target/release/herdr-projects`,
/// with `.exe` on Windows), not a test or debug binary that must never become
/// the user's command.
pub fn installable(binary: &Path) -> bool {
    binary.ends_with(Path::new("release").join(format!("{NAME}{}", std::env::consts::EXE_SUFFIX)))
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// A link that resolves to `binary`.
    Ours,
    Missing,
    /// A link that is dangling or points to another build inside Herdr's
    /// plugin folder: safe to replace.
    Stale(PathBuf),
    /// A file, or a link somewhere else (a checkout of the user's own): never touched.
    Foreign(String),
}

pub fn state(env: &Env, binary: &Path) -> State {
    let link = link_path(env);
    if std::fs::symlink_metadata(&link).is_err() {
        return State::Missing;
    }
    let target = match crate::platform::read_command_link(&link) {
        Ok(target) => target,
        Err(what) => return State::Foreign(what),
    };
    let target = link.parent().map(|dir| dir.join(&target)).unwrap_or(target);
    let binary = dunce::canonicalize(binary).unwrap_or_else(|_| binary.to_path_buf());
    match dunce::canonicalize(&target) {
        Ok(resolved) if resolved == binary => State::Ours,
        Err(_) => State::Stale(target),
        Ok(_) if target.starts_with(plugins_dir(env)) => State::Stale(target),
        Ok(_) => State::Foreign(format!("a link to {}", target.display())),
    }
}

/// Herdr's plugin installs live under `<herdr config dir>/plugins`.
fn plugins_dir(env: &Env) -> PathBuf {
    let config = crate::setup::herdr_config_path(env);
    config.parent().unwrap_or(Path::new("/")).join("plugins")
}

/// Makes the link when it is missing or stale; returns the state found.
pub fn ensure(env: &Env, binary: &Path) -> Result<State> {
    let found = state(env, binary);
    if matches!(found, State::Missing | State::Stale(_)) {
        let link = link_path(env);
        let dir = bin_dir(env);
        std::fs::create_dir_all(&dir).with_context(|| format!("could not create {}", dir.display()))?;
        // Link under a temporary name, then rename over: never a moment without a command.
        let tmp = dir.join(format!(".{}.{}", crate::platform::command_link_name(NAME), std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        crate::platform::link_command(binary, &tmp).with_context(|| format!("could not link {}", link.display()))?;
        if let Err(error) = std::fs::rename(&tmp, &link) {
            let _ = std::fs::remove_file(&tmp);
            return Err(error).with_context(|| format!("could not link {}", link.display()));
        }
    }
    Ok(found)
}

/// What a shell with `path_var` runs for `herdr-projects`, if anything (on
/// Windows through `PATHEXT`, so both `herdr-projects.exe` and our `.cmd` shim count).
pub fn resolves_to(path_var: &str) -> Option<PathBuf> {
    let pathext = std::env::var("PATHEXT").ok();
    std::env::split_paths(path_var)
        .flat_map(|dir| crate::platform::program_candidates(&dir, NAME, pathext.as_deref()))
        .find(|candidate| candidate.is_file())
}

/// The `doctor` line: `(ok, detail)` where `None` is a warning.
pub fn check(env: &Env, binary: &Path, path_var: &str, fix: bool) -> (Option<bool>, String) {
    let link = link_path(env);
    if !installable(binary) {
        return (None, format!("{} is not an installed build, so {} was not checked", binary.display(), link.display()));
    }
    let (found, fixed) = if fix {
        match ensure(env, binary) {
            Ok(found) => {
                let fixed = matches!(found, State::Missing | State::Stale(_));
                (found, fixed)
            }
            Err(error) => return (Some(false), format!("could not fix: {error:#}")),
        }
    } else {
        (state(env, binary), false)
    };
    let mut detail = match &found {
        State::Foreign(what) => return (None, format!("{} is {what}, so it was left alone; move it away and run `doctor --fix` to link this binary there", link.display())),
        State::Missing if !fix => return (None, format!("{} is missing; `doctor --fix` (or restarting Herdr) links it", link.display())),
        State::Stale(old) if !fix => return (None, format!("{} links {}, not this binary; `doctor --fix` (or restarting Herdr) relinks it", link.display(), old.display())),
        _ if fixed => format!("fixed: {} now links this binary", link.display()),
        _ => format!("{} links this binary", link.display()),
    };
    let dir = bin_dir(env);
    if !std::env::split_paths(path_var).any(|d| d == dir) {
        detail.push_str(&format!("; but {} is not on your PATH: add `export PATH=\"{}:$PATH\"` to your shell profile", dir.display(), dir.display()));
        return (None, detail);
    }
    // What the command found runs: through our link (on Windows a `.cmd`
    // shim, which canonicalising does not see through) or itself.
    let runs = |found: &Path| match crate::platform::read_command_link(found) {
        Ok(target) => found.parent().map(|dir| dir.join(&target)).unwrap_or(target),
        Err(_) => found.to_path_buf(),
    };
    match resolves_to(path_var) {
        Some(first) if dunce::canonicalize(runs(&first)).ok() != dunce::canonicalize(binary).ok() => {
            detail.push_str(&format!("; but your PATH finds {} first, which is another binary", first.display()));
            (None, detail)
        }
        _ => (Some(true), detail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, Env, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let binary = home.path().join(format!(".config/herdr/plugins/github/herdr-projects-abc/target/release/herdr-projects{}", std::env::consts::EXE_SUFFIX));
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, "#!/bin/sh\n").unwrap();
        (home, env, binary)
    }

    fn path_with(env: &Env) -> String {
        join_path(&[bin_dir(env), PathBuf::from("/usr/bin")])
    }

    fn join_path(dirs: &[PathBuf]) -> String {
        std::env::join_paths(dirs).unwrap().to_string_lossy().into_owned()
    }

    #[test]
    fn a_missing_link_is_made_and_then_ours() {
        let (_home, env, binary) = setup();
        assert_eq!(ensure(&env, &binary).unwrap(), State::Missing);
        assert_eq!(crate::platform::read_command_link(&link_path(&env)).unwrap(), binary);
        assert_eq!(ensure(&env, &binary).unwrap(), State::Ours);
        let (ok, detail) = check(&env, &binary, &path_with(&env), false);
        assert_eq!(ok, Some(true), "{detail}");
    }

    #[test]
    #[cfg(unix)]
    fn a_dangling_link_or_one_into_another_plugin_install_is_replaced() {
        let (home, env, binary) = setup();
        std::fs::create_dir_all(bin_dir(&env)).unwrap();
        std::os::unix::fs::symlink(home.path().join("gone/herdr-projects"), link_path(&env)).unwrap();
        assert!(matches!(ensure(&env, &binary).unwrap(), State::Stale(_)));
        assert_eq!(std::fs::read_link(link_path(&env)).unwrap(), binary);

        let other = home.path().join(".config/herdr/plugins/github/herdr-projects-old/target/release/herdr-projects");
        std::fs::create_dir_all(other.parent().unwrap()).unwrap();
        std::fs::write(&other, "").unwrap();
        std::fs::remove_file(link_path(&env)).unwrap();
        std::os::unix::fs::symlink(&other, link_path(&env)).unwrap();
        let (ok, detail) = check(&env, &binary, &path_with(&env), true);
        assert_eq!(ok, Some(true), "{detail}");
        assert!(detail.starts_with("fixed:"), "{detail}");
        assert_eq!(std::fs::read_link(link_path(&env)).unwrap(), binary);
    }

    #[test]
    #[cfg(unix)]
    fn a_file_or_a_link_to_the_users_own_checkout_is_never_touched() {
        let (home, env, binary) = setup();
        std::fs::create_dir_all(bin_dir(&env)).unwrap();
        std::fs::write(link_path(&env), "mine").unwrap();
        assert!(matches!(ensure(&env, &binary).unwrap(), State::Foreign(_)));
        assert_eq!(std::fs::read_to_string(link_path(&env)).unwrap(), "mine");
        let (ok, detail) = check(&env, &binary, &path_with(&env), true);
        assert_eq!(ok, None);
        assert!(detail.contains("left alone"), "{detail}");

        std::fs::remove_file(link_path(&env)).unwrap();
        let own = home.path().join("dev/herdr-projects/target/release/herdr-projects");
        std::fs::create_dir_all(own.parent().unwrap()).unwrap();
        std::fs::write(&own, "").unwrap();
        std::os::unix::fs::symlink(&own, link_path(&env)).unwrap();
        assert!(matches!(ensure(&env, &binary).unwrap(), State::Foreign(_)));
        assert_eq!(std::fs::read_link(link_path(&env)).unwrap(), own);
    }

    #[test]
    #[cfg(unix)]
    fn a_bin_dir_missing_from_path_is_a_warning_with_the_fix() {
        let (_home, env, binary) = setup();
        let (ok, detail) = check(&env, &binary, "/usr/bin", true);
        assert_eq!(ok, None);
        assert!(detail.contains("is not on your PATH: add `export PATH="), "{detail}");
        assert!(link_path(&env).is_symlink());
    }

    #[test]
    fn another_binary_earlier_on_path_is_named() {
        let (home, env, binary) = setup();
        let early = home.path().join("early");
        std::fs::create_dir_all(&early).unwrap();
        std::fs::write(early.join(format!("{NAME}{}", std::env::consts::EXE_SUFFIX)), "").unwrap();
        let path = join_path(&[early, bin_dir(&env), PathBuf::from("/usr/bin")]);
        let (ok, detail) = check(&env, &binary, &path, true);
        assert_eq!(ok, None);
        assert!(detail.contains("finds") && detail.contains("early"), "{detail}");
    }

    #[test]
    fn xdg_bin_home_wins_and_debug_builds_are_never_linked() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[("XDG_BIN_HOME", "/xdg/bin")]);
        assert_eq!(link_path(&env), PathBuf::from("/xdg/bin").join(crate::platform::command_link_name(NAME)));
        let (ok, _) = check(&env, Path::new("/src/target/debug/herdr-projects"), "", true);
        assert_eq!(ok, None);
        assert!(!Path::new("/xdg/bin").exists());
    }

    /// Runs scripts/link-command.sh for `checkout` with `home` as HOME.
    #[cfg(unix)]
    fn run_script(home: &Path, checkout: &Path) -> String {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/link-command.sh");
        let out = std::process::Command::new("sh")
            .arg(script)
            .arg(checkout)
            .env_clear()
            .env("HOME", home)
            .env("PATH", format!("{}:/usr/bin:/bin", home.join(".local/bin").display()))
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stderr).into_owned()
    }

    #[test]
    #[cfg(unix)]
    fn the_install_script_links_where_herdr_will_move_the_checkout() {
        let home = tempfile::tempdir().unwrap();
        let plugins = home.path().join(".config/herdr/plugins");
        let checkout = plugins.join(".tmp-install-12-34/checkout");
        let link = home.path().join(".local/bin/herdr-projects");
        // Herdr's folder for plugin id `herdr-projects`: slug plus sha256's first 12 hex digits.
        let final_binary = plugins.join("github/herdr-projects-b1278ffb803c/target/release/herdr-projects");

        // Anywhere but a Herdr install checkout: nothing.
        run_script(home.path(), &home.path().join("dev/herdr-projects"));
        assert!(std::fs::symlink_metadata(&link).is_err());

        let said = run_script(home.path(), &checkout);
        assert_eq!(std::fs::read_link(&link).unwrap(), final_binary, "{said}");
        assert!(said.contains("linked"), "{said}");

        // A dangling link elsewhere is replaced; a working one is left alone.
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(home.path().join("gone"), &link).unwrap();
        run_script(home.path(), &checkout);
        assert_eq!(std::fs::read_link(&link).unwrap(), final_binary);
        let own = home.path().join("own");
        std::fs::write(&own, "").unwrap();
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&own, &link).unwrap();
        assert!(run_script(home.path(), &checkout).contains("left"));
        assert_eq!(std::fs::read_link(&link).unwrap(), own);

        // A file is never touched.
        std::fs::remove_file(&link).unwrap();
        std::fs::write(&link, "mine").unwrap();
        assert!(run_script(home.path(), &checkout).contains("not a link"));
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "mine");
    }
}
