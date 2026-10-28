//! Other machines: why a machine did not answer, each with what to do on it; the door another machine asks (one line of
//! JSON, actions run where the task lives); and the SSH line that asks it, through a login shell.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::remote::why::{known_host, not_list, unanswered, why_not, Ended};
mod same_socket;

fn end(code: i32, stdout: &str, stderr: &str) -> Ended { Ended { code: Some(code), stdout: stdout.into(), stderr: stderr.into(), timed_out: false, error: String::new(), ms: 0 } }

#[test]
fn why_a_machine_did_not_answer_each_with_what_to_do() {
    let ends: Vec<(&str, Ended, &str, &str)> = vec![
        ("unknownName", end(255, "", "ssh: Could not resolve hostname vm-a: Name or service not known\n"), "connect", r"cannot connect \(its name is not known\)"),
        ("timedOut", end(255, "", "ssh: connect to host vm-a port 22: Connection timed out\n"), "connect", "connecting timed out"),
        ("refused", end(255, "", "ssh: connect to host vm-a port 22: Connection refused\n"), "connect", "it refused the connection"),
        ("hostKey", end(255, "", "@@@@@@@@@@@\n@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\n@@@@@@@@@@@\nHost key for vm-a has changed and you have requested strict checking.\nHost key verification failed.\n"), "connect", "its host key changed"),
        ("noKey", end(255, "", "me@vm-a: Permission denied (publickey).\n"), "login", "does not let the board in"),
        ("bash", end(127, "", "bash: line 1: herdr-projects: command not found\n"), "path", "herdr-projects is not found there"),
        ("dash", end(127, "", "sh: 1: herdr-projects: not found\n"), "path", "herdr-projects is not found there"),
        ("zsh", end(127, "", "zsh:1: command not found: herdr-projects\n"), "path", "herdr-projects is not found there"),
        ("fish", end(127, "", "fish: Unknown command: herdr-projects\nherdr-projects remote list\n^~~~~~~~~~~~^\n"), "path", "herdr-projects is not found there"),
        ("noNode", end(127, "", "env: 'node': No such file or directory\n"), "path", "node is not found there"),
        ("oldNode", end(1, "", "SyntaxError: Unexpected token 'with'\n    at compileSourceTextModule\n\nNode.js v18.19.1\n"), "path", r"its node is too old \(18\)"),
        ("banner", end(0, "Welcome to vm-a\nLast login: today\n", ""), "answer", "something else than the board's answer"),
        ("nothing", end(0, "", ""), "answer", "answered nothing"),
        ("error", end(1, "", "Error: cannot read the tasks\n"), "answer", "answered with an error"),
        ("slow", Ended { code: None, stdout: String::new(), stderr: String::new(), timed_out: true, error: String::new(), ms: 20000 }, "slow", "took too long"),
    ];
    for (name, e, kind, line) in &ends {
        let why = why_not(e, "vm-a", "me@vm-a");
        assert_eq!(why.s("kind"), *kind, "{name}");
        assert!(regex::Regex::new(line).unwrap().is_match(why.s("line")), "{name}: {}", why.s("line"));
        assert!(regex::Regex::new(r#"^Test it from this machine: ssh me@vm-a 'exec \$SHELL -lc "herdr-projects remote list"' \| tail -c 200$"#).unwrap().is_match(&text(why.arr("detail").last().unwrap())), "{name}: {}", why.at("detail"));
        assert!(why.s("line").chars().count() < 60, "{name}");
    }
    let bash = why_not(&ends[5].1, "vm-a", "me@vm-a");
    let d = bash.arr("detail").iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(d.contains("must be on the PATH that a login shell gets") && d.contains("~/.bashrc does not count") && d.contains("~/.profile") && d.contains(r#"ln -s "$(command -v herdr-projects)" ~/.local/bin/"#), "{d}");
    assert_eq!(bash.s("said"), "bash: line 1: herdr-projects: command not found");
    assert!(!why_not(&ends[3].1, "vm-a", "me@vm-a").s("said").contains("@@@"));
    assert!(why_not(&ends[3].1, "vm-a", "ssh://me@vm-a:2200").at("detail").to_string().contains("ssh-keygen -R '[vm-a]:2200' -f"));
    assert!(why_not(&ends[4].1, "vm-a", "me@vm-a").at("detail").to_string().contains("ssh-copy-id me@vm-a"));
    assert_eq!(["me@vm-a", "ssh://me@vm-a:22", "ssh://me@vm-a:2200", "vm-a", "ssh://[fd7a::1]:2200", "ssh://me@[fd7a::1]"].map(known_host), ["vm-a", "vm-a", "[vm-a]:2200", "vm-a", "[fd7a::1]:2200", "fd7a::1"]);
    assert!(text(&not_list(&json!({ "ok": false, "msg": "Unknown request list" }), "vm-a", "me@vm-a").arr("detail")[0]).contains("vm-a answered: Unknown request list"));
    assert_eq!(unanswered("vm-a", &why_not(&ends[2].1, "vm-a", "me@vm-a")), "vm-a did not answer: cannot connect (it refused the connection)");
    assert_eq!(unanswered("vm-a", &serde_json::Value::Null), "vm-a did not answer");
}

#[test]
fn real_programs_the_door_answers_one_line_of_json_and_actions_run_where_the_task_lives() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "board.machines": "exedev@vm-a, vm-b" }), ..Default::default() });
    let id = w.add("Remote task");
    let door = |args: &[&str]| -> serde_json::Value {
        let o = std::process::Command::new(env!("CARGO_BIN_EXE_herdr-projects")).arg("remote").args(args).current_dir(&w.root).env("HERDR_SOCKET_PATH", "/nonexistent").env("HERDR_BIN_PATH", "/bin/false").output().unwrap();
        let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
        assert_eq!(out.lines().count(), 1, "one line: {out}");
        serde_json::from_str(&out).unwrap()
    };
    let pack = |v: serde_json::Value| sw::util::base64(v.to_string().as_bytes());
    let list = door(&["list"]);
    assert_eq!((list.arr("tasks")[0].s("id"), list.arr("tasks")[0].s("state")), (id.as_str(), "idle"));
    assert!(list.arr("tasks")[0].at("snap").b("folder"));
    assert_eq!(door(&["do", &pack(json!({ "action": "pin", "id": id }))]), json!({ "ok": true, "msg": "proj/1: pinned" }));
    assert_eq!(task(&id).at("pinned"), &json!(true));
    assert_eq!(door(&["do", &pack(json!({ "action": "nope", "id": id }))]).at("ok"), &json!(false));
    let disp = door(&["dispatch", &pack(json!({ "text": "Added from afar" }))]);
    assert_eq!(disp.at("ok"), &json!(true)); assert!(disp.s("msg").contains("added"), "{disp}");
    let found = sw::remote::machines();
    assert_eq!(found.iter().map(|m| (m.target.clone(), m.label.clone())).collect::<Vec<_>>(), vec![("exedev@vm-a".to_string(), "vm-a".to_string()), (format!("{}@vm-b", sw::store::files::me()), "vm-b".to_string())]);
    let (cmd, args) = sw::remote::ssh_command(&sw::remote::Machine { target: "exedev@vm-a".into(), session: "work".into(), ..Default::default() }, "herdr-projects remote list", false);
    assert_eq!(cmd, "ssh"); assert!(args.iter().any(|a| a == "BatchMode=yes"));
    assert_eq!(args.last().unwrap(), r#"exec /bin/sh -c 'export HERDR_SESSION=work; exec "${SHELL:-/bin/sh}" -lc "$0"' 'herdr-projects remote list'"#);
    let (_, at) = sw::remote::attach_command(&sw::remote::Machine { target: "exedev@vm-a".into(), ..Default::default() }, "w4:p1; rm -rf /");
    assert!(at.iter().any(|a| a == "-t")); assert_eq!(at.last().unwrap(), r#"exec /bin/sh -c 'exec "${SHELL:-/bin/sh}" -lc "$0"' 'herdr agent attach w4:p1rm-rf'"#);
}

/// The line sshd hands a machine's login shell, run as sshd runs it (`<shell> -c <line>`, that shell as $SHELL) by
/// every shell this machine has: fish too, when it is here. The command arrives whole, its session with it.
#[test]
fn real_programs_the_ssh_line_reads_alike_in_every_login_shell() {
    let w = world_with(Opts::default());
    let line = sw::remote::login_line("work", r#"printf "[%s] [%s] [%s]" "$HERDR_SESSION" "two  spaces" eyJhIjoxfQ==; printf "\n""#);
    let path = std::env::var("PATH").unwrap_or_default();
    let found: Vec<std::path::PathBuf> = ["sh", "dash", "bash", "zsh", "fish"].iter().filter_map(|s| std::env::split_paths(&path).map(|d| d.join(s)).find(|p| p.is_file())).collect();
    assert!(!found.is_empty());
    for shell in found {
        let o = std::process::Command::new(&shell).arg("-c").arg(&line).env_clear().env("HOME", &w.base).env("PATH", &path).env("SHELL", &shell).stdin(std::process::Stdio::null()).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&o.stdout), "[work] [two  spaces] [eyJhIjoxfQ==]\n", "{}: {}", shell.display(), String::from_utf8_lossy(&o.stderr));
    }
}

/// ssh's shared connection is a socket, and a socket's path may not pass 103 bytes on macOS: ssh names it with 57 of
/// them (`%C` and a dot and 16 more while it starts), so it lives in a short folder of this user's own, whatever the
/// board's folder is. A folder open to others is not used.
#[test]
fn the_ssh_connection_socket_fits_a_long_home_folder() {
    use std::os::unix::fs::PermissionsExt;
    let w = world();
    unsafe { std::env::set_var("HERDR_PROJECTS_HOME", w.base.join("Users/alexandrsukhryn/.herdr-projects")) };
    let path = || { let (_, a) = sw::remote::ssh_command(&sw::remote::Machine { target: "me@vm-a".into(), ..Default::default() }, "true", false); a.iter().find_map(|x| x.strip_prefix("ControlPath=").map(String::from)).unwrap() };
    let p = path();
    let short = format!("{}/hp-{}/ssh/%C", std::env::var("HERDR_PROJECTS_TMP").unwrap(), sw::agents::scratch::uid());
    assert_eq!(p, short, "out of the board's folder, whatever its length");
    assert!(format!("/tmp/hp-{}/ssh/", u32::MAX).len() + 57 <= 103, "where it lives outside a test, for any user");
    let dir = std::path::Path::new(&p).parent().unwrap();
    assert_eq!(std::fs::metadata(dir).unwrap().permissions().mode() & 0o777, 0o700);
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(path(), sw::store::files::data_dir().join("ssh-%C").display().to_string());
}
