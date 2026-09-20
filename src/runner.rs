//! Every external command (herdr, git, gh, ssh, scp, rsync, sh) goes through `Runner`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub env_remove: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub stdin: Option<String>,
    pub timeout: Duration,
    /// Spawn in its own process group and kill the whole group on timeout.
    pub own_group: bool,
}

impl Cmd {
    pub fn new(program: impl Into<String>, timeout: Duration) -> Self {
        Cmd {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            env_remove: Vec::new(),
            cwd: None,
            stdin: None,
            timeout,
            own_group: false,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn env_remove(mut self, key: impl Into<String>) -> Self {
        self.env_remove.push(key.into());
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn stdin(mut self, text: impl Into<String>) -> Self {
        self.stdin = Some(text.into());
        self
    }

    pub fn own_group(mut self) -> Self {
        self.own_group = true;
        self
    }

    /// The command as one line; the scripted fake matches on it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn display(&self) -> String {
        let mut line = self.program.clone();
        for arg in &self.args {
            line.push(' ');
            line.push_str(arg);
        }
        line
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output {
    /// `None` when the process was killed (timeout or signal).
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

impl Output {
    pub fn success(&self) -> bool {
        self.code == Some(0) && !self.timed_out
    }

    /// stderr when it has text, else stdout, trimmed; for error messages.
    pub fn error_text(&self) -> String {
        if self.timed_out {
            return "timed out".to_string();
        }
        let text = if self.stderr.trim().is_empty() {
            self.stdout.trim()
        } else {
            self.stderr.trim()
        };
        text.to_string()
    }
}

pub trait Runner {
    /// `Err` means the command could not be spawned at all (for example the
    /// program is missing). A non-zero exit or a timeout is an `Ok(Output)`.
    fn run(&self, cmd: &Cmd) -> Result<Output>;

    /// Ordered results for independent commands. Scripted runners stay serial;
    /// the real runner overlaps waits. Callers bound each batch to eight commands.
    fn run_batch(&self, commands: &[Cmd]) -> Vec<Result<Output>> {
        commands.iter().map(|cmd| self.run(cmd)).collect()
    }


    /// One JSON line to a herdr socket, one line back. The single exception to
    /// "talk to herdr through its CLI" (client decision during the build):
    /// herdr 0.9.1 has no CLI command for `agent.view.set` / `agent.view.clear`.
    fn socket_request(&self, socket: &Path, line: &str, timeout: Duration) -> Result<String>;
}

pub struct RealRunner;

const POLL: Duration = Duration::from_millis(20);

impl Runner for RealRunner {
    fn run_batch(&self, commands: &[Cmd]) -> Vec<Result<Output>> {
        commands.chunks(8).flat_map(|batch| std::thread::scope(|scope| {
            let workers: Vec<_> = batch.iter().map(|cmd| scope.spawn(move || self.run(cmd))).collect();
            workers.into_iter().map(|worker| worker.join().unwrap_or_else(|_| Err(anyhow::anyhow!("command worker panicked")))).collect::<Vec<_>>()
        })).collect()
    }

    fn run(&self, cmd: &Cmd) -> Result<Output> {
        let mut command = Command::new(&cmd.program);
        command.args(&cmd.args);
        for key in &cmd.env_remove {
            command.env_remove(key);
        }
        for (key, value) in &cmd.env {
            command.env(key, value);
        }
        if let Some(cwd) = &cmd.cwd {
            command.current_dir(cwd);
        }
        command
            .stdin(if cmd.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        if cmd.own_group {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }

        let mut child = command
            .spawn()
            .with_context(|| format!("could not run `{}`", cmd.program))?;

        // Readers and the writer run on their own threads so a full pipe in
        // either direction cannot deadlock against the deadline loop below.
        let stdin_thread = child.stdin.take().zip(cmd.stdin.clone()).map(|(mut pipe, text)| {
            std::thread::spawn(move || {
                let _ = pipe.write_all(text.as_bytes());
            })
        });
        let stdout_thread = child.stdout.take().map(read_all);
        let stderr_thread = child.stderr.take().map(read_all);

        let deadline = Instant::now() + cmd.timeout;
        let mut timed_out = false;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break Some(status);
            }
            if Instant::now() >= deadline {
                timed_out = true;
                kill(&mut child, cmd.own_group);
                break child.wait().ok();
            }
            std::thread::sleep(POLL);
        };

        if let Some(thread) = stdin_thread {
            let _ = thread.join();
        }
        let stdout = stdout_thread.map(join_text).unwrap_or_default();
        let stderr = stderr_thread.map(join_text).unwrap_or_default();

        Ok(Output {
            code: if timed_out {
                None
            } else {
                status.and_then(|s| s.code())
            },
            stdout,
            stderr,
            timed_out,
        })
    }

    fn socket_request(&self, socket: &Path, line: &str, timeout: Duration) -> Result<String> {
        socket_round_trip(socket, line, timeout)
    }
}

fn socket_round_trip(socket: &Path, line: &str, timeout: Duration) -> Result<String> {
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixStream;
    let mut stream = UnixStream::connect(socket)
        .with_context(|| format!("could not connect to {}", socket.display()))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply)
}

fn read_all<R: Read + Send + 'static>(mut pipe: R) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        buf
    })
}

fn join_text(thread: std::thread::JoinHandle<Vec<u8>>) -> String {
    String::from_utf8_lossy(&thread.join().unwrap_or_default()).into_owned()
}

fn kill(child: &mut std::process::Child, own_group: bool) {
    if own_group {
        // The child is its group's leader, so its pid is the pgid. Grandchildren
        // hold the pipes open; killing only the child would leave readers hanging.
        let _ = Command::new("/bin/kill")
            .args(["-TERM", "--", &format!("-{}", child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        std::thread::sleep(Duration::from_millis(200));
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::cell::RefCell;

    type Matcher = Box<dyn Fn(&Cmd) -> bool>;

    /// A scripted runner: the first rule whose matcher accepts the command
    /// answers it. Every command is recorded, matched or not.
    #[derive(Default)]
    pub struct FakeRunner {
        rules: RefCell<Vec<(Matcher, Box<dyn Fn(&Cmd) -> Result<Output>>)>>,
        pub calls: RefCell<Vec<Cmd>>,
        pub batches: RefCell<Vec<usize>>,
        /// (socket, request line) of every socket request.
        pub socket_requests: RefCell<Vec<(PathBuf, String)>>,
    }

    impl FakeRunner {
        pub fn new() -> Self {
            Self::default()
        }

        /// Answer commands whose display line contains `needle`.
        pub fn on(&self, needle: &str, output: Output) -> &Self {
            let needle = needle.to_string();
            self.rules.borrow_mut().push((
                Box::new(move |cmd| cmd.display().contains(&needle)),
                Box::new(move |_| Ok(output.clone())),
            ));
            self
        }

        pub fn on_fn(
            &self,
            matcher: impl Fn(&Cmd) -> bool + 'static,
            answer: impl Fn(&Cmd) -> Result<Output> + 'static,
        ) -> &Self {
            self.rules
                .borrow_mut()
                .push((Box::new(matcher), Box::new(answer)));
            self
        }

        pub fn count(&self, needle: &str) -> usize {
            self.calls
                .borrow()
                .iter()
                .filter(|cmd| cmd.display().contains(needle))
                .count()
        }
    }

    pub fn ok(stdout: &str) -> Output {
        Output {
            code: Some(0),
            stdout: stdout.to_string(),
            ..Output::default()
        }
    }

    pub fn fail(code: i32, stderr: &str) -> Output {
        Output {
            code: Some(code),
            stderr: stderr.to_string(),
            ..Output::default()
        }
    }

    pub fn timeout() -> Output {
        Output {
            timed_out: true,
            ..Output::default()
        }
    }

    impl Runner for FakeRunner {
        fn run_batch(&self, commands: &[Cmd]) -> Vec<Result<Output>> {
            self.batches.borrow_mut().push(commands.len());
            commands.iter().map(|cmd| self.run(cmd)).collect()
        }

        fn run(&self, cmd: &Cmd) -> Result<Output> {
            self.calls.borrow_mut().push(cmd.clone());
            for (matcher, answer) in self.rules.borrow().iter() {
                if matcher(cmd) {
                    return answer(cmd);
                }
            }
            anyhow::bail!("FakeRunner: no rule for `{}`", cmd.display())
        }

        fn socket_request(&self, socket: &Path, line: &str, _timeout: Duration) -> Result<String> {
            self.socket_requests.borrow_mut().push((socket.to_path_buf(), line.to_string()));
            Ok(r#"{"id":"hp","result":{"type":"agent_view","active":true}}"#.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_commands_in_a_batch_overlap_and_keep_result_order() {
        let dir = tempfile::tempdir().unwrap();
        let left = dir.path().join("left").to_string_lossy().into_owned();
        let right = dir.path().join("right").to_string_lossy().into_owned();
        let script = "touch \"$1\"; while [ ! -e \"$2\" ]; do sleep 0.01; done; printf '%s' \"$3\"";
        let commands = [
            Cmd::new("sh", Duration::from_secs(5)).args(["-c", script, "sh", &left, &right, "left"]),
            Cmd::new("sh", Duration::from_secs(5)).args(["-c", script, "sh", &right, &left, "right"]),
        ];
        let results = RealRunner.run_batch(&commands);
        assert!(results.iter().all(|r| r.as_ref().is_ok_and(Output::success)));
        assert_eq!(results[0].as_ref().unwrap().stdout, "left");
        assert_eq!(results[1].as_ref().unwrap().stdout, "right");
    }

    #[test]
    fn captures_output_and_exit_code() {
        let out = RealRunner
            .run(&Cmd::new("sh", Duration::from_secs(5)).args(["-c", "echo hi; echo err >&2; exit 3"]))
            .unwrap();
        assert_eq!(out.code, Some(3));
        assert_eq!(out.stdout, "hi\n");
        assert_eq!(out.stderr, "err\n");
        assert!(!out.success());
    }

    #[test]
    fn passes_stdin() {
        let out = RealRunner
            .run(&Cmd::new("cat", Duration::from_secs(5)).stdin("hello"))
            .unwrap();
        assert_eq!(out.stdout, "hello");
    }

    #[test]
    fn missing_program_is_an_error() {
        assert!(
            RealRunner
                .run(&Cmd::new("hp-no-such-program", Duration::from_secs(1)))
                .is_err()
        );
    }

    #[test]
    fn times_out_a_chatty_child() {
        // `yes` fills the pipe far past its buffer; the reader threads keep it
        // drained so the deadline still fires.
        let start = Instant::now();
        let out = RealRunner
            .run(&Cmd::new("yes", Duration::from_millis(300)))
            .unwrap();
        assert!(out.timed_out);
        assert!(!out.success());
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(out.stdout.len() > 65_536);
    }

    #[test]
    fn group_kill_reaches_grandchildren() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("survived");
        let script = format!("(sleep 2; touch '{}') & wait", marker.display());
        let start = Instant::now();
        let out = RealRunner
            .run(
                &Cmd::new("sh", Duration::from_millis(300))
                    .args(["-c", &script])
                    .own_group(),
            )
            .unwrap();
        assert!(out.timed_out);
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(2300));
        assert!(!marker.exists(), "grandchild outlived the group kill");
    }
}
