//! A bounded, read-only host probe: one absolute executable, JSON on stdout, no shell or agent process.
use super::Reading;
use crate::store::files::data_dir;
use std::os::unix::{fs::OpenOptionsExt, process::CommandExt};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn read(program: &str) -> Result<Reading, String> {
    if !std::path::Path::new(program).is_absolute() { return Err("board.admission must be an absolute executable path".into()); }
    let path = data_dir().join(format!("admission-probe-{}.json", crate::util::random_hex(12)));
    struct Output(std::path::PathBuf);
    impl Drop for Output { fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); } }
    let _output = Output(path.clone());
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let file = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).map_err(|e| e.to_string())?;
    let mut command = Command::new(program);
    command.stdin(Stdio::null()).stdout(file).stderr(Stdio::null());
    unsafe { command.pre_exec(|| {
        if libc::setsid() < 0 { return Err(std::io::Error::last_os_error()); }
        let limit = libc::rlimit { rlim_cur: 16384, rlim_max: 16384 };
        if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 { return Err(std::io::Error::last_os_error()); }
        Ok(())
    }); }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return Err("Host probe failed".into()),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL); }
                let _ = child.wait();
                return Err("Host probe timed out".into());
            }
        }
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.len() > 16384 { return Err("Host probe output exceeded 16 KiB".into()); }
    serde_json::from_slice(&bytes).map_err(|_| "Host probe returned invalid JSON".into())
}
