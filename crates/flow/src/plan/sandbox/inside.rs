//! Is this process inside a sandbox: the plan writer's own, or any other? A program the board starts in the read-only
//! mount carries `HERDR_PROJECTS_BOXED=1`, and so does everything it runs (its turn-end hook, the board's commands it
//! calls). On macOS the system is asked too (`sandbox_check`), for a sandbox the board did not make. What runs in one
//! cannot do the board's own work: the worktree cannot be made writable there, and macOS allows no sandbox inside
//! another, so a reviewer could not be started in one of its own. Such work, asked for in the board's own mount, is
//! handed to the keeper outside it (runner/outside.rs); a Herdr that itself runs in a sandbox keeps doing its own.

/// The mark a program started in the mount carries, and everything it starts.
pub const MARK: &str = "HERDR_PROJECTS_BOXED";
/// In the board's own mount: its keeper, outside, is there to start what is asked for.
pub fn boxed() -> bool { std::env::var(MARK).as_deref() == Ok("1") }
/// macOS's own check (libsystem_sandbox, in every Mac process): 1 for a sandboxed process, 0 for one that is not; with
/// no operation named (SANDBOX_FILTER_NONE) it asks only that. It is looked up when it is asked for, not linked, so
/// this builds, and is checked, everywhere: elsewhere there is no such function, and so no sandbox it could see.
type Check = unsafe extern "C" fn(libc::pid_t, *const libc::c_char, libc::c_int, ...) -> libc::c_int;
pub fn os_says() -> bool {
    let f = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"sandbox_check".as_ptr()) };
    if f.is_null() { return false; }
    let check: Check = unsafe { std::mem::transmute::<*mut libc::c_void, Check>(f) };
    unsafe { check(libc::getpid(), std::ptr::null(), 0) == 1 }
}
/// Tests run the board's jobs in their own process (and may themselves run in a sandbox): there only the mark counts.
pub fn inside() -> bool { boxed() || (!crate::runner::inline() && os_says()) }
