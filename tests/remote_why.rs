//! Other machines that do not answer: the rest of the reasons and what to do, the answer read whatever a shell prints
//! around it, asking over an SSH stand-in, and `herdr-projects machines` (this machine as a login shell finds it, then
//! each machine). The boards' side of it: remote_board.rs.
use crate::common;
use crate::far;
use common::*;
use far::{theirs, Far};
use serde_json::json;
use sw::json::*;
use sw::remote::why::{read_answer, why_not, Ended};

fn end(code: i32, stdout: &str, stderr: &str) -> Ended { Ended { code: Some(code), stdout: stdout.into(), stderr: stderr.into(), ..Default::default() } }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
const LIST: &str = r#"{"host":"vm-a","version":"0.2.0","tasks":[]}"#;

#[test]
fn the_other_reasons_a_machine_did_not_answer_each_with_what_to_do() {
    let ends: Vec<(&str, Ended, &str, &str)> = vec![
        ("password", end(255, "", "me@vm-a: Permission denied (publickey,password).\n"), "login", "does not let the board in"),
        ("noNodeMac", end(127, "", "env: node: No such file or directory\n"), "path", "node is not found there"),
        ("noSsh", Ended { error: "spawn ssh ENOENT".into(), ..Default::default() }, "connect", "ssh could not be started"),
    ];
    for (name, e, kind, line) in &ends {
        let why = why_not(e, "vm-a", "me@vm-a");
        assert_eq!(why.s("kind"), *kind, "{name}");
        assert!(has(why.s("line"), line), "{name}: {}", why.s("line"));
        assert!(has(&text(why.arr("detail").last().unwrap()), r#"^Test it from this machine: ssh me@vm-a 'exec \$SHELL -lc "herdr-projects remote list"' \| tail -c 200$"#), "{name}");
        assert!(why.s("line").chars().count() < 60, "{name}");
    }
    let detail = |e: &Ended, target: &str| why_not(e, "vm-a", target).arr("detail").iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(detail(&end(127, "", "env: 'node': No such file or directory\n"), "me@vm-a").contains(r#"ln -s "$(command -v node)""#));
    assert!(has(&detail(&end(255, "", "me@vm-a: Permission denied (publickey).\n"), "me@vm-a"), r"(?s)batch mode.*ssh-copy-id me@vm-a"));
    let host_key = end(255, "", "@@@@@@@@@@@\n@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\n@@@@@@@@@@@\nHost key verification failed.\n");
    assert!(has(&detail(&host_key, "ssh://me@vm-a:2200"), r"ssh-keygen -R '\[vm-a\]:2200' -f .*known_hosts"));
}

#[test]
fn the_answer_is_read_whatever_a_shells_start_up_files_print_around_it() {
    let list: serde_json::Value = serde_json::from_str(LIST).unwrap();
    for out in [format!("{LIST}\n"), format!("Welcome to vm-a\r\n\r\n{LIST}\n"), format!("Welcome\n{LIST}\nlogout\nBye {{for now}}\n\n"), format!("{{\"motd\":\"hello\"}}\n{LIST}\n[1, 2]\n\"text\"\n42\n"),
        format!("{LIST}\n{{\"motd\":\"Goodbye\"}}\n"), format!("{LIST}\n{}", "Goodbye\n".repeat(500))] {
        assert_eq!(read_answer(&out), Some(list.clone()), "{out:?}");
    }
    assert_eq!(read_answer("{\"ok\":true,\"msg\":\"\"}\n{\"motd\":\"Goodbye\"}\n"), Some(json!({ "ok": true, "msg": "" })));
    assert_eq!(read_answer("{\"motd\":\"hello\"}\nbye\n"), Some(json!({ "motd": "hello" })), "none of the board's: said as \"not its tasks\"");
    for none in ["", "Welcome\n", "[1,2]\n", "{not json}\n", "null\n"] { assert_eq!(read_answer(none), None, "{none:?}"); }
}

#[test]
fn real_programs_asking_over_the_ssh_stand_in_keeps_the_exit_code_and_stderr_and_gives_up_on_a_machine_that_hangs() {
    let w = world();
    let far = Far::new(&w.base);
    far.ends("good", "list", &format!("Welcome\n{LIST}\nlogout\n"), "warning: something on stderr\n", 0);
    far.ends("nopath", "list", "Welcome\n", "bash: line 1: herdr-projects: command not found\n", 127);
    far.ends("nonode", "list", "", "env: 'node': No such file or directory\n", 127);
    far.ends("down", "list", "", "ssh: connect to host vm port 22: Connection refused\n", 255);
    far.ends("nokey", "list", "", "me@vm: Permission denied (publickey).\n", 255);
    far.ends("banner", "list", "Welcome to the machine\n", "", 0);
    far.hangs("hangs", "list", 3);
    let ask = |n: &str, ms: u64| sw::remote::ask(&sw::remote::Machine { label: n.into(), target: format!("me@{n}"), ..Default::default() }, "list", None, ms);
    assert_eq!(ask("good", 20000).unwrap(), serde_json::from_str::<serde_json::Value>(LIST).unwrap());
    let failed: Vec<(String, String, i64)> = ["nopath", "nonode", "down", "nokey", "banner"].iter().map(|n| { let why = ask(n, 20000).unwrap_err(); (n.to_string(), why.t("kind"), why.i("code")) }).collect();
    assert_eq!(failed, [("nopath", "path", 127), ("nonode", "path", 127), ("down", "connect", 255), ("nokey", "login", 255), ("banner", "answer", 0)].map(|(a, b, c)| (a.to_string(), b.to_string(), c)));
    let nopath = ask("nopath", 20000).unwrap_err();
    assert_eq!((nopath.s("line"), nopath.s("said")), ("herdr-projects is not found there", "bash: line 1: herdr-projects: command not found"));
    assert_eq!(ask("banner", 20000).unwrap_err().s("said"), "Welcome to the machine");
    assert_eq!(ask("hangs", 40).unwrap_err().s("kind"), "slow");
    assert_eq!(far.asked().iter().filter(|a| a.0 == "good").map(|a| a.1.clone()).collect::<Vec<_>>(), vec!["list"]);
}

/// A login shell that answers as the test says: what it was run with and what it was given is written down.
fn fake_shell(w: &World, out: &str, err: &str, code: i32) -> std::path::PathBuf {
    let d = w.base.join("shell");
    std::fs::create_dir_all(&d).unwrap();
    for (n, v) in [("out", out.to_string()), ("err", err.to_string()), ("code", code.to_string())] { std::fs::write(d.join(n), v).unwrap(); }
    let script = d.join("sh");
    std::fs::write(&script, format!("#!/bin/sh\nD='{}'\nenv | sed 's/=.*//' | sort | tr '\\n' ' ' > \"$D/env\"\nprintf '%s' \"$2\" > \"$D/ran\"\ncat \"$D/err\" >&2\ncat \"$D/out\"\nexit \"$(cat \"$D/code\")\"\n", d.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[test]
fn real_programs_herdr_projects_machines_this_machine_as_a_bare_login_shell_then_each_machine() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a, vm-b" }), ..Default::default() });
    let far = Far::new(&w.base);
    let mut archived = theirs(2); archived["archivedAt"] = json!(1);
    far.tasks("vm-a", "0.2.0", json!([theirs(1), archived]));
    far.ends("vm-b", "list", "", "env: 'node': No such file or directory\n", 127);
    let version = sw::store::files::VERSION;
    let shell = fake_shell(&w, &format!("banner\n{}\n", json!({ "ok": true, "host": "here", "version": version })), "", 0);
    let had = std::env::var("SHELL").ok();
    unsafe { std::env::set_var("SHELL", &shell) };
    let r = sw::remote::check::check(20000);
    assert_eq!(read(&w.base.join("shell/ran")), "herdr-projects remote hello");
    let env = read(&w.base.join("shell/env"));
    for k in ["HOME", "LOGNAME", "PATH", "SHELL", "USER"] { assert!(env.split(' ').any(|x| x == k), "{k}: {env}"); }
    assert!(!env.split(' ').any(|x| x.starts_with("HERDR") || x.starts_with("GIT_") || x == "TERM"), "nothing of the terminal it was typed in: {env}");
    assert_eq!((r.at("here").at("ok"), r.at("here").s("version")), (&json!(true), version));
    let ms: Vec<(String, bool, String, serde_json::Value)> = r.arr("machines").iter().map(|m| (m.t("label"), m.b("ok"), if m.b("version") { m.t("version") } else { m.at("why").t("kind") }, m.at("tasks").clone())).collect();
    assert_eq!(ms, vec![("vm-a".into(), true, "0.2.0".into(), json!(1)), ("vm-b".into(), false, "path".into(), serde_json::Value::Null)]);
    let said = sw::remote::check::say(&r);
    assert!(has(&said, &format!(r"this machine: a login shell \(.+\) finds herdr-projects {} · 1000 GB free\n", regex::escape(version))), "{said}");
    assert!(has(&said, r"vm-a +answers · herdr-projects 0\.2\.0 · 1 task  \(.*@vm-a\)\n"), "{said}");
    assert!(has(&said, r"vm-b +does not answer: node is not found there  \(.*@vm-b\)\n    A login shell on vm-b finds `herdr-projects` but not `node`"), "{said}");
    assert!(has(&said, r"    What came back:\n      env: 'node': No such file or directory"), "{said}");
    // this machine itself, when a login shell does not find the program: said with the fix, by the same reading
    fake_shell(&w, "", "bash: line 1: herdr-projects: command not found\n", 127);
    let lost = sw::remote::check::here(20000);
    assert_eq!((lost.at("ok"), lost.at("why").s("kind")), (&json!(false), "path"));
    assert!(has(&lost.at("why").arr("detail").iter().map(text).collect::<Vec<_>>().join("\n"), r"(?s)must be on the PATH that a login shell gets.*Test it from another machine: ssh "));
    assert!(has(&sw::remote::check::say(&json!({ "here": lost, "machines": [], "off": false })), r"(?s)this machine: other machines cannot ask it: herdr-projects is not found there\n.*No other machines\. Add one with"));
    // an older one that is found is found
    fake_shell(&w, "{\"ok\":false,\"msg\":\"Unknown request hello\"}\n", "", 0);
    assert_eq!(sw::remote::check::here(20000).at("ok"), &json!(true));
    match had { Some(s) => unsafe { std::env::set_var("SHELL", s) }, None => unsafe { std::env::remove_var("SHELL") } }
    // the door's own answer to the check
    let hello = sw::remote::door("hello", &json!({})).unwrap();
    assert_eq!(hello.as_object().unwrap().keys().cloned().collect::<Vec<_>>(), vec!["ok", "host", "version"]);
}
