//! [real programs] A job the board starts and leaves to run is waited for when it ends: it does not stay a zombie of
//! the long-lived keeper or board that started it until that ends, and its marker reads as gone (#212).
use crate::common;
use common::*;
use std::time::{Duration, Instant};

/// A process state letter (`Z`: a zombie, ended and nobody waited for it), None once it is gone.
fn state(pid: u32) -> Option<char> {
    #[cfg(target_os = "linux")]
    {
        let s = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        return s[s.rfind(')')? + 2..].chars().next();
    }
    #[cfg(not(target_os = "linux"))]
    {
        let output = std::process::Command::new("ps")
            .args(["-o", "state=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        String::from_utf8_lossy(&output.stdout).trim().chars().next()
    }
}
fn until(done: impl Fn() -> bool) -> bool { let end = Instant::now() + Duration::from_secs(20); while Instant::now() < end { if done() { return true; } std::thread::sleep(Duration::from_millis(10)); } done() }

#[test]
fn real_programs_a_detached_job_that_ended_is_reaped_and_its_marker_reads_as_gone() {
    let _w = world();
    unsafe { std::env::remove_var("HERDR_PROJECTS_INLINE") }
    // A user's notify command, and a job of no task's (this test program, asked for a test no test is named).
    let sh = sw::runner::detached_sh("exit 0", &[]);
    let loose = sw::housekeeping::spawn_loose(&["--list", "no_test_has_this_name"]);
    unsafe { std::env::set_var("HERDR_PROJECTS_INLINE", "1") }
    for pid in [sh, loose] {
        assert!(pid > 0, "it did not start");
        assert!(until(|| state(pid).is_none()), "{pid} is left as {:?}", state(pid));
        assert!(!sw::store::files::pid_alive(pid as i64));
    }
}

#[test]
fn real_programs_a_detached_job_nobody_waits_for_stays_a_zombie_that_reads_as_alive() {
    // What every job was before: started detached, and its `Child` dropped.
    let mut c = std::process::Command::new("sh");
    c.args(["-c", "exit 0"]).stdin(std::process::Stdio::null());
    sw::runner::detach(&mut c);
    let pid = c.spawn().unwrap().id();
    assert!(until(|| state(pid) == Some('Z')), "{pid} is {:?}", state(pid));
    assert!(sw::store::files::pid_alive(pid as i64), "a zombie answers kill 0");
    // Waited for here, so the test leaves nothing behind.
    assert_eq!(unsafe { libc::waitpid(pid as i32, std::ptr::null_mut(), 0) }, pid as i32);
    assert_eq!(state(pid), None);
}
