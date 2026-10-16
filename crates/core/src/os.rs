//! What differs between Linux and macOS on one side and Windows on the other, in one place: processes and their
//! signals, file locks, file identity and times, links, private files and the terminal. Every other file asks here
//! instead of reaching for libc or std::os::unix, so the program builds and runs on Windows too.
use std::fs::{File, Metadata, OpenOptions};
use std::path::Path;
use std::process::Command;

// ---- processes ------------------------------------------------------------------------------------------------------

/// Whether a process of this id runs (one we may not signal counts as running).
#[cfg(unix)]
pub fn pid_alive(pid: i64) -> bool {
    if pid <= 0 { return false; }
    let r = unsafe { libc::kill(pid as libc::pid_t, 0) };
    r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}
#[cfg(windows)]
pub fn pid_alive(pid: i64) -> bool {
    if pid <= 0 || pid > u32::MAX as i64 { return false; }
    win::alive(pid as u32)
}

/// Asks a process to end (SIGTERM). Windows has no such request: the process is ended.
pub fn term(pid: i64) -> bool {
    if pid <= 0 { return false; }
    #[cfg(unix)]
    { unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) == 0 } }
    #[cfg(windows)]
    { win::end(pid as u32) }
}
/// Ends a process at once (SIGKILL).
pub fn kill(pid: i64) -> bool {
    if pid <= 0 { return false; }
    #[cfg(unix)]
    { unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) == 0 } }
    #[cfg(windows)]
    { win::end(pid as u32) }
}
/// Asks a process and everything it started to end: its process group when it leads one (`detach`), else itself.
/// On Windows the whole tree under it is ended.
pub fn term_group(pid: i64) {
    if pid <= 0 { return; }
    #[cfg(unix)]
    unsafe { if libc::kill(-(pid as libc::pid_t), libc::SIGTERM) != 0 { libc::kill(pid as libc::pid_t, libc::SIGTERM); } }
    #[cfg(windows)]
    win::end_tree(pid as u32);
}
/// Ends a process group (`detach`) at once. On Windows the whole tree under the process.
pub fn kill_group(pid: i64) {
    if pid <= 0 { return; }
    #[cfg(unix)]
    unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL); }
    #[cfg(windows)]
    win::end_tree(pid as u32);
}
/// Ends every process in our own process group (a job's helpers), then this one.
pub fn kill_own_group_and_exit() -> ! {
    #[cfg(unix)]
    unsafe { libc::kill(0, libc::SIGKILL); libc::_exit(0) }
    #[cfg(windows)]
    { win::end_tree(std::process::id()); std::process::exit(0) }
}
/// A child started in a group of its own, so it and what it starts can be ended together (`kill_group`), and it
/// outlives a terminal that closes.
pub fn detach(c: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe { c.pre_exec(|| { libc::setsid(); Ok(()) }); }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
}
/// Calls `f` when the process is asked to end (SIGTERM, SIGINT, SIGHUP). Windows: nothing is called, the process
/// just ends.
pub fn on_end_signal(f: extern "C" fn(i32)) {
    #[cfg(unix)]
    unsafe { for s in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] { libc::signal(s, f as *const () as usize); } }
    #[cfg(windows)]
    let _ = f;
}
/// Calls `f` when the process is asked to end (SIGTERM only).
pub fn on_term_signal(f: extern "C" fn(i32)) {
    #[cfg(unix)]
    unsafe { libc::signal(libc::SIGTERM, f as *const () as usize); }
    #[cfg(windows)]
    let _ = f;
}
/// The machine's load over the last minute, five and fifteen; None where it is not known (Windows).
pub fn load_average() -> Option<[f64; 3]> {
    #[cfg(unix)]
    {
        let mut l = [0f64; 3];
        let n = unsafe { libc::getloadavg(l.as_mut_ptr(), 3) };
        (n == 3).then_some(l)
    }
    #[cfg(windows)]
    { None }
}
/// This machine's name.
pub fn hostname() -> String {
    #[cfg(unix)]
    {
        let mut b = [0u8; 256];
        unsafe { libc::gethostname(b.as_mut_ptr() as *mut libc::c_char, b.len()); }
        String::from_utf8_lossy(&b[..b.iter().position(|c| *c == 0).unwrap_or(0)]).into_owned()
    }
    #[cfg(windows)]
    { std::env::var("COMPUTERNAME").unwrap_or_default() }
}
/// The user's login name, from the system; empty when it is not known.
pub fn login_name() -> String {
    #[cfg(unix)]
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() { String::new() } else { std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned() }
    }
    #[cfg(windows)]
    { std::env::var("USERNAME").unwrap_or_default() }
}
/// Whether this file descriptor (0 stdin, 1 stdout, 2 stderr) is a terminal.
pub fn is_tty(fd: i32) -> bool {
    use std::io::IsTerminal;
    match fd { 0 => std::io::stdin().is_terminal(), 1 => std::io::stdout().is_terminal(), _ => std::io::stderr().is_terminal() }
}

/// A command line of the user's or the repo's (setup, check, a notify command), run by a shell: `sh -c` (`-lc` for a
/// login shell). On Windows the `sh` of Git for Windows when there is one, so the same command works on every machine;
/// else cmd.exe.
pub fn shell(cmd: &str, login: bool) -> Command {
    #[cfg(unix)]
    { let mut c = Command::new("sh"); c.args([if login { "-lc" } else { "-c" }, cmd]); c }
    #[cfg(windows)]
    {
        match git_sh() {
            Some(sh) => { let mut c = Command::new(sh); c.args([if login { "-lc" } else { "-c" }, cmd]); c }
            None => {
                use std::os::windows::process::CommandExt;
                let mut c = Command::new(std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into()));
                c.arg("/d").arg("/s").arg("/c").raw_arg(format!("\"{cmd}\""));
                c
            }
        }
    }
}
/// Git for Windows' `sh.exe`: on PATH, or beside the `git` that is (`<Git>\cmd\git.exe` -> `<Git>\bin\sh.exe`).
#[cfg(windows)]
fn git_sh() -> Option<std::path::PathBuf> {
    static SH: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    SH.get_or_init(|| {
        if let Some(sh) = on_path("sh") { return Some(sh); }
        let git = on_path("git")?;
        let root = git.parent()?.parent()?;
        [root.join("bin").join("sh.exe"), root.join("usr").join("bin").join("sh.exe")].into_iter().find(|p| p.is_file())
    }).clone()
}
/// The program of this name on PATH (with Windows' PATHEXT endings), if there is one.
pub fn on_path(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    let ends: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into()).split(';').filter(|e| !e.is_empty()).map(|e| e.to_ascii_lowercase()).collect()
    } else { vec![String::new()] };
    std::env::split_paths(&path).find_map(|dir| ends.iter().map(|e| dir.join(format!("{name}{e}"))).find(|p| p.is_file()))
}

// ---- files ----------------------------------------------------------------------------------------------------------

/// Takes the file's exclusive lock: waits for it (`wait`), or only when it is free now. Let go when the file closes.
pub fn lock(f: &File, wait: bool) -> bool { if wait { f.lock().is_ok() } else { f.try_lock().is_ok() } }
pub fn unlock(f: &File) { let _ = f.unlock(); }
/// When the file last changed, in milliseconds since 1970.
pub fn mtime_ms(m: &Metadata) -> f64 {
    #[cfg(unix)]
    { use std::os::unix::fs::MetadataExt; m.mtime() as f64 * 1000.0 + m.mtime_nsec() as f64 / 1e6 }
    #[cfg(windows)]
    { m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0) }
}
/// When the file last changed, in nanoseconds since 1970: what a cache compares.
pub fn mtime_ns(m: &Metadata) -> i128 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos() as i128).unwrap_or(0)
}
/// The file's identity: its disk and its number there. On Windows that is not in its metadata, so (0, 0): callers
/// compare it together with the file's time and size.
pub fn file_id(m: &Metadata) -> (u64, u64) {
    #[cfg(unix)]
    { use std::os::unix::fs::MetadataExt; (m.dev(), m.ino()) }
    #[cfg(windows)]
    { let _ = m; (0, 0) }
}
/// Its permission bits (0 on Windows, which has none).
pub fn mode(m: &Metadata) -> u32 {
    #[cfg(unix)]
    { use std::os::unix::fs::MetadataExt; m.mode() }
    #[cfg(windows)]
    { let _ = m; 0 }
}
/// Whether the two are on one disk (where a rename moves a folder in one step). Unknown on Windows: compared by drive.
pub fn same_disk(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    { use std::os::unix::fs::MetadataExt; matches!((std::fs::metadata(a), std::fs::metadata(b)), (Ok(x), Ok(y)) if x.dev() == y.dev()) }
    #[cfg(windows)]
    {
        let root = |p: &Path| std::fs::canonicalize(p).ok().and_then(|p| p.components().next().map(|c| c.as_os_str().to_ascii_lowercase()));
        matches!((root(a), root(b)), (Some(x), Some(y)) if x == y)
    }
}
/// Whether the open file is still the one at this path (none took its place).
pub fn same_file(f: &File, path: &Path) -> bool {
    #[cfg(unix)]
    { match (f.metadata(), std::fs::metadata(path)) { (Ok(a), Ok(b)) => file_id(&a) == file_id(&b), _ => false } }
    // Windows: an open file cannot be renamed over or removed while we hold it.
    #[cfg(windows)]
    { let _ = f; path.exists() }
}
/// A link at `at` to `target`. On Windows a folder gets a directory link (which needs developer mode or admin rights).
pub fn symlink(target: &Path, at: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    { std::os::unix::fs::symlink(target, at) }
    #[cfg(windows)]
    {
        let dir = at.parent().map(|p| p.join(target)).unwrap_or_else(|| target.to_path_buf()).is_dir();
        if dir { std::os::windows::fs::symlink_dir(target, at) } else { std::os::windows::fs::symlink_file(target, at) }
    }
}
/// Options that make a file only its owner may read (0600). Windows keeps a user's files in their own profile already.
pub fn private(o: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(unix)]
    { use std::os::unix::fs::OpenOptionsExt; o.mode(0o600) }
    #[cfg(windows)]
    { o }
}
/// Options that refuse to open the file through a link at its path (O_NOFOLLOW).
pub fn no_follow(o: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(unix)]
    { use std::os::unix::fs::OpenOptionsExt; o.custom_flags(libc::O_NOFOLLOW) }
    #[cfg(windows)]
    { o }
}
/// Makes a file runnable (chmod 755). Nothing to do on Windows.
pub fn make_runnable(p: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)) }
    #[cfg(windows)]
    { let _ = p; Ok(()) }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    const PROCESS_TERMINATE: u32 = 0x0001;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    const ERROR_ACCESS_DENIED: i32 = 5;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
        fn TerminateProcess(process: *mut c_void, code: u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    pub fn alive(pid: u32) -> bool {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() { return std::io::Error::last_os_error().raw_os_error() == Some(ERROR_ACCESS_DENIED); }
            let mut code = 0u32;
            let ok = GetExitCodeProcess(h, &mut code) != 0;
            CloseHandle(h);
            ok && code == STILL_ACTIVE
        }
    }
    pub fn end(pid: u32) -> bool {
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if h.is_null() { return false; }
            let ok = TerminateProcess(h, 1) != 0;
            CloseHandle(h);
            ok
        }
    }
    /// The process and everything under it.
    pub fn end_tree(pid: u32) {
        let mut c = std::process::Command::new("taskkill");
        c.args(["/T", "/F", "/PID", &pid.to_string()]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        if !c.status().map(|s| s.success()).unwrap_or(false) { end(pid); }
    }
}
