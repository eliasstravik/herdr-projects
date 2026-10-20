//! Is this process inside a sandbox: the plan writer's own, or any other? A program the board starts in the read-only
//! mount carries `HERDR_PROJECTS_BOXED=1`, and so does everything it runs (its turn-end hook, the board's commands it
//! calls). On macOS the system is asked too (`sandbox_check`), for a sandbox the board did not make. What runs in one
//! cannot do the board's own work: the worktree cannot be made writable there, and macOS allows no sandbox inside
//! another, so a reviewer could not be started in one of its own. Such work is handed to the keeper (runner/outside.rs).

/// The mark a program started in the mount carries, and everything it starts.
pub const MARK: &str = "HERDR_PROJECTS_BOXED";
fn marked() -> bool { std::env::var(MARK).as_deref() == Ok("1") }
#[cfg(target_os = "macos")]
fn os_says() -> bool {
    extern "C" { fn sandbox_check(pid: libc::pid_t, operation: *const libc::c_char, kind: libc::c_int) -> libc::c_int; }
    unsafe { sandbox_check(libc::getpid(), std::ptr::null(), 0) > 0 }
}
#[cfg(not(target_os = "macos"))]
fn os_says() -> bool { false }
/// Tests run the board's jobs in their own process (and may themselves run in a sandbox): there only the mark counts.
pub fn inside() -> bool { marked() || (!crate::runner::inline() && os_says()) }
