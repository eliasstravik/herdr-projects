//! What the keeper takes from a login shell when herdr's server started it without: the SSH agent (`SSH_AUTH_SOCK`,
//! `SSH_AGENT_PID`) for its `git fetch` and `git push` over SSH, only for a process with no agent of its own, only a
//! socket that is there, and with only its names remembered in `env.json`. And herdr, found without PATH (herdr_find).
use crate::common;
use common::*;
use sw::login_env::{remembered, ssh_agent};

/// herdr found when PATH lacks it: the usual install spots, then a login shell's PATH.
mod herdr_find;

fn pair(k: &str, v: &str) -> (String, String) { (k.to_string(), v.to_string()) }

#[test]
fn a_keeper_started_without_an_ssh_agent_takes_the_login_shells_and_a_process_with_its_own_keeps_it() {
    let w = world();
    let out = format!("Welcome\nPATH=/a/bin\nSSH_AUTH_SOCK={}\nSSH_AGENT_PID=4242\nHOME=/home/me\n", w.socket);
    assert_eq!(ssh_agent(&out, |_| false), vec![pair("SSH_AUTH_SOCK", &w.socket), pair("SSH_AGENT_PID", "4242")]);
    assert!(ssh_agent(&out, |k| k == "SSH_AUTH_SOCK").is_empty(), "an agent of its own is kept");
    assert!(ssh_agent(&out, |k| k == "SSH_AGENT_PID").is_empty(), "an agent of its own is kept");
    assert!(ssh_agent("PATH=/x\nSSH_AUTH_SOCK=\n", |_| false).is_empty());
}

#[test]
fn an_ssh_agent_whose_socket_is_not_there_is_not_taken() {
    let w = world();
    let gone = w.base.join("gone.sock").display().to_string();
    assert!(ssh_agent(&format!("SSH_AUTH_SOCK={gone}\nSSH_AGENT_PID=4242\n"), |_| false).is_empty());
    let file = w.base.join("a.txt"); std::fs::write(&file, "x").unwrap();
    assert!(ssh_agent(&format!("SSH_AUTH_SOCK={}\n", file.display()), |_| false).is_empty(), "a file that is no socket");
}

#[test]
fn env_json_remembers_the_names_of_the_ssh_agent_and_of_gh_login_never_their_values() {
    let w = world();
    let out = format!("PATH=/a/bin\nSSH_AUTH_SOCK={}\nSSH_AGENT_PID=4242\nGH_TOKEN=s3cret\n", w.socket);
    let c = serde_json::Value::Object(remembered(&out, "/fallback"));
    assert_eq!(c["ssh"], serde_json::json!(["SSH_AUTH_SOCK", "SSH_AGENT_PID"]));
    assert_eq!(c["gh"], serde_json::json!(["GH_TOKEN"]));
    assert_eq!(c["PATH"], "/a/bin");
    let text = c.to_string();
    for v in [w.socket.as_str(), "4242", "s3cret"] { assert!(!text.contains(v), "{v} written down: {text}"); }
    assert_eq!(serde_json::Value::Object(remembered("HOME=/h\n", "/fallback"))["ssh"], serde_json::json!([]));
    assert_eq!(serde_json::Value::Object(remembered("HOME=/h\n", "/fallback"))["PATH"], "/fallback");
}
