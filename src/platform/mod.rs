//! What differs between Unix and Windows, behind one interface: the round
//! trip to Herdr's socket, detached and grouped processes, ignoring the
//! terminal's interrupts, links, the clipboard and the system opener. Callers
//! never need `cfg`; each function names its Unix and Windows behaviour.

use std::fs::Metadata;
use std::path::Path;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as imp;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

/// One JSON line to Herdr's socket, one line back: a Unix socket at `socket`,
/// or on Windows the named pipe `\\.\pipe\<socket>` (Herdr's own naming).
pub use imp::socket_round_trip;

/// Configures `command` to outlive whatever started this process: a new
/// session (`setsid`) on Unix, `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`
/// on Windows. The caller sets stdio and spawns.
pub use imp::detach;

/// Configures `command` to start its own process group, so
/// [`kill_tree`] reaches everything it starts.
pub use imp::own_group;

/// Ends `child`, and with `own_group` everything it started: the process
/// group (TERM, then KILL) on Unix, `taskkill /T /F` on Windows.
pub use imp::kill_tree;

/// While alive, this process ignores the terminal's interrupt keys (SIGINT
/// and SIGQUIT on Unix, Ctrl+C on Windows), so a foreground child handles
/// them alone. Restored on drop. Make it after spawning the child, which keeps
/// the default handling.
pub use imp::IgnoreInterrupts;

/// The file name of a command link called `name` in a `bin` folder:
/// `name` on Unix, `name.cmd` on Windows.
pub use imp::command_link_name;

/// Makes `link` run `binary`: a symbolic link on Unix, a `.cmd` shim that
/// forwards every argument on Windows (symbolic links there need Developer
/// Mode or admin rights).
pub use imp::link_command;

/// What a command link made by [`link_command`] runs, as written (it may be
/// relative to the link's folder); `Err` says what `link` is instead.
pub use imp::read_command_link;

/// Makes `link` a link to the directory `target`: a symbolic link on Unix, a
/// directory junction on Windows (no special rights needed).
pub use imp::link_dir;

/// The directory a link made by [`link_dir`] points to.
pub use imp::read_dir_link;

/// Removes a link made by [`link_dir`] or [`link_command`], not what it points to.
pub use imp::remove_link;

/// Makes a project's `CLAUDE.md` read its `AGENTS.md`: a relative symbolic
/// link on Unix, a one-line `@AGENTS.md` import on Windows (Claude Code
/// follows it; a copy would go stale).
pub use imp::link_claude_md;

/// Whether `CLAUDE.md` is what [`link_claude_md`] makes.
pub use imp::is_claude_md_link;

/// Clipboard tools to try in order, with their arguments; each reads the
/// text on standard input.
pub use imp::CLIPBOARD;

/// The program and leading arguments that open a URL or a file with the
/// user's default application; the target is the last argument.
pub use imp::OPENER;

/// Whether `meta` (from `symlink_metadata`) is a link of any kind: a symbolic
/// link, or on Windows also a junction or any other reparse point. Safety
/// checks treat all of them alike and never follow them.
pub fn is_link(meta: &Metadata) -> bool {
    imp::is_link(meta)
}

/// Whether `path` itself (not followed) is a link of any kind.
pub fn is_link_path(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| is_link(&m))
}

/// A regular file that is not a link of any kind.
pub fn is_plain_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && !is_link(&m))
}

/// A directory that is not a link of any kind.
pub fn is_plain_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !is_link(&m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_files_and_dirs_are_not_links() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        std::fs::write(&file, "x").unwrap();
        assert!(is_plain_file(&file) && !is_plain_dir(&file) && !is_link_path(&file));
        assert!(is_plain_dir(dir.path()) && !is_plain_file(dir.path()));
        assert!(!is_plain_file(&dir.path().join("missing")) && !is_link_path(&dir.path().join("missing")));
    }

    #[test]
    fn a_dir_link_is_a_link_and_reads_back_and_removes_alone() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("SKILL.md"), "x").unwrap();
        let link = dir.path().join("link");
        link_dir(&target, &link).unwrap();
        assert!(is_link_path(&link) && !is_plain_dir(&link));
        assert_eq!(read_dir_link(&link).unwrap(), target);
        assert!(link.join("SKILL.md").is_file());
        remove_link(&link).unwrap();
        assert!(std::fs::symlink_metadata(&link).is_err());
        assert!(target.join("SKILL.md").is_file(), "removing the link removed its target");
    }

    #[test]
    fn a_command_link_reads_back_its_binary() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("release").join("hp");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, "").unwrap();
        let link = dir.path().join(command_link_name("hp"));
        link_command(&binary, &link).unwrap();
        assert_eq!(read_command_link(&link).unwrap(), binary);
        let file = dir.path().join(command_link_name("other"));
        std::fs::write(&file, "something else\n").unwrap();
        assert!(read_command_link(&file).is_err());
    }

    #[test]
    fn claude_md_link_is_recognised_and_a_plain_file_is_not() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "# Agents\n").unwrap();
        let claude = dir.path().join("CLAUDE.md");
        assert!(!is_claude_md_link(&claude));
        link_claude_md(&claude).unwrap();
        assert!(is_claude_md_link(&claude));
        std::fs::remove_file(&claude).unwrap();
        std::fs::write(&claude, "# my own\n").unwrap();
        assert!(!is_claude_md_link(&claude));
    }

    #[test]
    fn a_missing_socket_is_an_error_not_a_hang() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("no-such-herdr.sock");
        let start = std::time::Instant::now();
        assert!(socket_round_trip(&socket, "{}", std::time::Duration::from_secs(2)).is_err());
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }
}
