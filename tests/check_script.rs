//! [real programs] The check's own scripts (`scripts/check.sh`, `scripts/run-tests.sh`) as a machine starts them: an
//! empty home, with no build cache, gets past its start and says why it stopped, never with no word; and the cores are
//! shared between the checks running now (`scripts/share-cores.sh`): all of them for a lone check, half for one of
//! two, never fewer than one, those that died not counted, your own numbers first, and none of it where the build
//! cache would see it.
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// A copy of the check's scripts in a git repo of its own, an empty home, a machine of three cores, and a `cargo` that
/// writes down what it was given and stops the check (exit 7, without a word), so only the check's start runs.
struct Bench { base: PathBuf, others: Vec<Child> }
impl Bench {
    fn new(name: &str) -> Bench {
        let base = std::env::temp_dir().join(format!("sw-check-script-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let proj = base.join("proj");
        std::fs::create_dir_all(proj.join("scripts")).unwrap();
        std::fs::create_dir_all(base.join("home")).unwrap();
        std::fs::create_dir_all(base.join("bin")).unwrap();
        for f in ["check.sh", "run-tests.sh", "build-cache.sh", "share-cores.sh"] {
            std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts").join(f), proj.join("scripts").join(f)).unwrap();
        }
        let b = Bench { base, others: vec![] };
        b.stub("cargo", &format!("echo \"$* | ${{CARGO_BUILD_JOBS-unset}} | ${{RUSTC_WRAPPER:-none}}\" >> '{}'\nexit 7", b.saw_file().display()));
        b.stub("rustc", "echo rustc 0.0.0");
        b.stub("getconf", "echo 3");
        b.git(&["init", "-q"]);
        b
    }
    fn stub(&self, name: &str, body: &str) {
        let p = self.base.join("bin").join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    }
    fn git(&self, a: &[&str]) {
        let o = Command::new("git").args(a).current_dir(self.base.join("proj")).env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1").output().unwrap();
        assert!(o.status.success(), "git {a:?}: {}", String::from_utf8_lossy(&o.stderr));
    }
    fn home(&self) -> PathBuf { self.base.join("home") }
    fn saw_file(&self) -> PathBuf { self.base.join("cargo-saw") }
    fn running(&self) -> PathBuf { self.home().join(".cache/herdr-projects/running") }
    /// `sh <script> <args>` with nothing but this home, a plain PATH and `env`: its exit code, stderr, and what the
    /// stand-in cargo was given, a line a call (empty when the check never got to it).
    fn sh(&self, args: &[&str], env: &[(&str, &str)]) -> (i32, String, String) {
        let _ = std::fs::remove_file(self.saw_file());
        let o = Command::new("/bin/sh").args(args).current_dir(self.base.join("proj")).env_clear().env("HOME", self.home())
            .env("PATH", format!("{}:/usr/bin:/bin", self.base.join("bin").display())).envs(env.iter().copied()).output().unwrap();
        let saw = std::fs::read_to_string(self.saw_file()).unwrap_or_default();
        (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stderr).into_owned(), saw.trim().to_string())
    }
    fn check(&self, env: &[(&str, &str)]) -> (i32, String, String) { self.sh(&["scripts/check.sh", "--all"], env) }
    /// Another check running now: a live process, written down as a running check writes itself down.
    fn another(&mut self) -> u32 {
        let c = Command::new("sleep").arg("60").spawn().unwrap();
        let pid = c.id();
        self.others.push(c);
        std::fs::create_dir_all(self.running()).unwrap();
        std::fs::write(self.running().join(pid.to_string()), started(pid)).unwrap();
        pid
    }
}
impl Drop for Bench {
    fn drop(&mut self) {
        for c in &mut self.others { let _ = c.kill(); let _ = c.wait(); }
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

/// When a process started, as the scripts tell one process from another that was given the same number later.
fn started(pid: u32) -> String {
    String::from_utf8_lossy(&Command::new("ps").env("LC_ALL", "C").args(["-o", "lstart=", "-p", &pid.to_string()]).output().unwrap().stdout).into_owned()
}

/// Every file under a folder, by its path from there.
fn files(dir: &Path) -> Vec<String> {
    let mut out = vec![];
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() { out.extend(files(&p).into_iter().map(|f| format!("{}/{f}", e.file_name().to_string_lossy()))); }
        else { out.push(e.file_name().to_string_lossy().into_owned()); }
    }
    out
}

#[test]
fn real_programs_a_lone_check_starts_in_an_empty_home_on_every_core_and_says_why_it_stopped() {
    let b = Bench::new("empty");
    let (code, err, saw) = b.check(&[]);
    // three cores, no other check: all three; no build cache: cargo as it is
    assert_eq!(saw, "build -j 3 --profile quick --quiet | unset | none", "the check stopped before building: {err}");
    assert_eq!(code, 7);
    assert!(err.contains("The check stopped while building the program (exit 7)."), "{err}");
    // it wrote itself down while it ran, and took itself off when it stopped: no file is left in the home
    assert!(b.running().is_dir());
    assert_eq!(files(&b.home()), Vec::<String>::new());
}

#[test]
fn real_programs_the_cores_are_shared_between_the_checks_running_now_and_one_that_died_is_not_counted() {
    let mut b = Bench::new("shared");
    let one = b.another();
    let (_, err, saw) = b.check(&[]);
    assert_eq!(saw, "build -j 2 --profile quick --quiet | unset | none", "one of two takes half, rounded up: {err}");
    // one that died without taking itself off, and a number given again to a process that is not the check: not counted
    let gone = { let mut c = Command::new("true").spawn().unwrap(); c.wait().unwrap(); c.id() };
    std::fs::write(b.running().join(gone.to_string()), "Thu Jan  1 00:00:00 1970\n").unwrap();
    std::fs::write(b.running().join(std::process::id().to_string()), "Thu Jan  1 00:00:00 1970\n").unwrap();
    let (_, _, saw) = b.check(&[]);
    assert_eq!(saw, "build -j 2 --profile quick --quiet | unset | none");
    assert_eq!(files(&b.running()), vec![one.to_string()], "the dead ones are taken off, the live one kept");
    // three more: five checks on three cores, and still one core each
    for _ in 0..3 { b.another(); }
    let (_, _, saw) = b.check(&[]);
    assert_eq!(saw, "build -j 1 --profile quick --quiet | unset | none");
}

#[test]
fn real_programs_your_own_job_count_wins_and_reaches_cargo_as_its_jobs_not_where_the_build_cache_keys_on_it() {
    let mut b = Bench::new("yours");
    b.another();
    let (_, err, saw) = b.check(&[("CARGO_BUILD_JOBS", "5")]);
    assert_eq!(saw, "build -j 5 --profile quick --quiet | unset | none", "{err}");
    // a word cargo would not take as a count is the share
    let (_, _, saw) = b.check(&[("CARGO_BUILD_JOBS", "many")]);
    assert_eq!(saw, "build -j 2 --profile quick --quiet | unset | none");
}

#[test]
fn real_programs_the_tests_run_on_the_share_when_they_start_or_on_your_own_count() {
    let mut b = Bench::new("tests");
    // a cargo that builds one test program, which writes down how many threads it was given
    let prog = b.base.join("bin/main-0");
    b.stub("main-0", &format!("[ \"$1\" = --list ] && {{ echo 'a::t: test'; exit 0; }}\necho \"$RUST_TEST_THREADS\" >> '{}'\necho 'test result: ok. 1 passed; 0 failed;'",
        b.base.join("threads").display()));
    b.stub("cargo", &format!("echo \"$*\" >> '{}'\ncase \"$*\" in *--no-run*) echo '{{\"profile\":{{\"test\":true}},\"executable\":\"{}\"}}' ;; *) echo 'test result: ok. 0 passed;' ;; esac",
        b.saw_file().display(), prog.display()));
    let threads = |b: &Bench| { let t = std::fs::read_to_string(b.base.join("threads")).unwrap(); let _ = std::fs::remove_file(b.base.join("threads")); t.trim().to_string() };
    let (code, err, saw) = b.sh(&["scripts/run-tests.sh"], &[]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(saw.lines().collect::<Vec<_>>(), [
        "test -j 3 --profile quick --quiet --no-run --message-format=json-render-diagnostics",
        "test -j 3 --profile quick --quiet --doc --",
    ]);
    assert_eq!(threads(&b), "3");
    b.another();
    let (_, _, saw) = b.sh(&["scripts/run-tests.sh"], &[("RUST_TEST_THREADS", "1")]);
    assert!(saw.starts_with("test -j 2 "), "{saw}");
    assert_eq!(threads(&b), "1", "your own count of threads wins");
}

#[test]
fn real_programs_a_check_that_stops_before_it_starts_says_why_once() {
    let b = Bench::new("refused");
    let (code, err, saw) = {
        let o = Command::new("/bin/sh").args(["scripts/check.sh", "--nope"]).current_dir(b.base.join("proj")).env_clear()
            .env("HOME", b.home()).env("PATH", "/usr/bin:/bin").output().unwrap();
        (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stderr).into_owned(), std::fs::read_to_string(b.saw_file()).unwrap_or_default())
    };
    assert_eq!((code, err.as_str(), saw.as_str()), (2, "check does not know --nope. It takes: --all, --fresh, --since <commit>, and names of tests.\n", ""));
}
