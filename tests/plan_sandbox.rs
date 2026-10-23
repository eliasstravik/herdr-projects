//! The read-only mount a plan's writer runs in, on each platform (plan/sandbox/tool.rs): bubblewrap's command on Linux,
//! `sandbox-exec` and its Seatbelt profile on macOS, the programs whose own sandbox is turned off inside it, what the
//! board says where no mount can be made (no apt advice on a Mac), doctor's line, and a plan step started that way;
//! inside such a mount, a probe that is never kept and a runner that is started from outside instead. And the real thing: a write into the guarded folders is refused while one elsewhere works (on macOS always; on Linux
//! where bubblewrap can be used).
use crate::common;
use common::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use sw::doctor::plan::{check, Mount};
use sw::json::*;
use sw::plan::sandbox::{boxed_command, tool::profile, Tool};

const PLAN: &str = "# Plan\n\n1. Add f.txt with one line.\n";
fn real(p: &Path) -> String { std::fs::canonicalize(p).unwrap().display().to_string() }

#[test]
fn the_seatbelt_profile_denies_writes_under_the_real_paths_only() {
    let w = world();
    let (tree, git) = (w.base.join("tree"), w.base.join("odd \"git\" dir"));
    std::fs::create_dir_all(&tree).unwrap();
    std::fs::create_dir_all(&git).unwrap();
    // A folder named by a link is guarded by where it really is (on a Mac: /var/… is /private/var/…).
    let link = w.base.join("link");
    std::os::unix::fs::symlink(&tree, &link).unwrap();
    let p = profile(&[link.clone(), tree.clone(), git.clone()]);
    assert_eq!(p, format!("(version 1)(allow default)(deny file-write* (subpath \"{}\") (subpath \"{}\"))", real(&tree), real(&git).replace('"', "\\\"")));
    assert_eq!(profile(&[]), "(version 1)(allow default)", "nothing to guard denies nothing: never every write");
    assert_eq!(Tool::SandboxExec.wrap(&[tree.clone()]), vec!["sandbox-exec".to_string(), "-p".into(), profile(&[tree.clone()])]);
    let t = tree.display().to_string();
    assert_eq!(Tool::Bwrap.wrap(&[tree.clone()]), ["bwrap", "--dev-bind", "/", "/", "--ro-bind", &t, &t, "--die-with-parent", "--"].map(String::from).to_vec());
    assert_eq!((Tool::of("sandbox-exec"), Tool::of("bwrap"), Tool::of("none")), (Some(Tool::SandboxExec), Some(Tool::Bwrap), None));
    assert_eq!(Tool::native(), if cfg!(target_os = "macos") { Tool::SandboxExec } else { Tool::Bwrap });
}

#[test]
fn each_platform_names_its_own_tool_and_a_program_s_own_sandbox_is_off_inside_seatbelt() {
    let mac = Tool::SandboxExec.no_mount("proj/1", "writer");
    assert!(mac.contains("`sandbox-exec`") && mac.contains("set plan.approve none --task proj/1") && !mac.contains("apt") && !mac.contains("bubblewrap"), "{mac}");
    let linux = Tool::Bwrap.no_mount("proj/1", "writer");
    assert!(linux.contains("needs bubblewrap") && linux.contains("sudo apt install bubblewrap"), "{linux}");
    assert!(Tool::SandboxExec.not_started("codex", "proj/1").starts_with("codex did not start in its read-only mount (sandbox-exec)."));
    assert_eq!((Tool::Bwrap.denied(), Tool::SandboxExec.denied()), ("Read-only file system", "Operation not permitted"));
    // macOS allows no sandbox inside another: Codex's and Claude Code's own are turned off; ours is the guard.
    let ask: Vec<String> = vec!["--no-daemon".into()];
    assert_eq!(Tool::SandboxExec.unnested("codex", &ask), vec!["--no-daemon", "-c", "sandbox_mode=\"danger-full-access\""]);
    let full: Vec<String> = vec!["--dangerously-bypass-approvals-and-sandbox".into()];
    assert_eq!(Tool::SandboxExec.unnested("codex", &full), full, "already off");
    assert_eq!(Tool::SandboxExec.unnested("claude", &[]), vec!["--settings", "{\"sandbox\":{\"enabled\":false}}"]);
    assert_eq!((Tool::SandboxExec.unnested("pi", &ask), Tool::Bwrap.unnested("codex", &ask), Tool::Bwrap.unnested("claude", &[])), (ask.clone(), ask.clone(), vec![]));
    // Claude Code takes one settings file, and the board's hooks are in it: the sandbox goes into a private copy of it.
    let w = world();
    let hooks = w.base.join("hook-agent.json");
    std::fs::write(&hooks, r#"{"hooks":{"Stop":[]}}"#).unwrap();
    let got = Tool::SandboxExec.unnested("claude", &["--dangerously-skip-permissions".into(), "--settings".into(), hooks.display().to_string()]);
    let copy = w.base.join("hook-agent.sandbox-off.json");
    assert_eq!(got, vec!["--dangerously-skip-permissions".to_string(), "--settings".into(), copy.display().to_string()]);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&copy).unwrap()).unwrap(), json!({ "hooks": { "Stop": [] }, "sandbox": { "enabled": false } }));
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(&copy).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(std::fs::read_to_string(&hooks).unwrap(), r#"{"hooks":{"Stop":[]}}"#, "the board's own file is left as it is");
    // doctor: a problem only where plans are asked for.
    let none = |used| check(&Mount { tool: Tool::SandboxExec, why: Some("sandbox-exec cannot make the mount here".into()), used });
    assert!(none(true)[0].problem && none(true)[0].fix.contains("outside any sandbox"));
    assert!(!none(false)[0].problem && none(false)[0].what.contains("no setting asks for it"));
    let ok = check(&Mount { tool: Tool::Bwrap, why: None, used: true });
    assert!(!ok[0].problem && ok[0].what.starts_with("Plan approval can run here") && ok[0].what.ends_with("(bwrap)"));
}

#[test]
fn a_plan_step_on_macos_starts_its_writer_under_sandbox_exec_with_the_worktree_and_its_git_folder_guarded() {
    let w = world_with(Opts { repo: true, settings: json!({ "plan.approve": "you", "worker.agent": "codex", "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "sandbox-exec") };
    w.patch(|s| s["pids"] = json!(true));
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Written." }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    let a = w.pane_of(&t).at("agent").clone();
    assert_eq!((a.at("boxed"), a.s("kind")), (&json!(true), "codex"));
    let tree = PathBuf::from(t.s("worktree"));
    let git = sw::plan::lock::git_dir_of(&tree).unwrap();
    let want = format!("sandbox-exec -p '(version 1)(allow default)(deny file-write* (subpath \"{}\") (subpath \"{}\"))' codex", real(&tree), real(&git));
    let full = boxed_command(&t, "x", "codex", &["--dangerously-bypass-approvals-and-sandbox".into()]);
    assert!(full.starts_with("env HERDR_PROJECTS_AGENT=x ") && full.contains(&want) && full.ends_with("codex --dangerously-bypass-approvals-and-sandbox"), "{full}");
    assert!(boxed_command(&t, "x", "codex", &["--no-daemon".into()]).ends_with("codex --no-daemon -c 'sandbox_mode=\"danger-full-access\"'"));
    assert!(boxed_command(&t, "x", "claude", &[]).ends_with("claude --settings '{\"sandbox\":{\"enabled\":false}}'"));
    // Where it cannot be used, the task says so in macOS's words.
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "none") };
    assert!(sw::plan::sandbox::no_mount("proj/1", "writer").contains(if cfg!(target_os = "macos") { "sandbox-exec" } else { "bubblewrap" }));
}

#[test]
fn inside_a_sandbox_a_probe_is_never_kept_and_a_runner_asked_for_is_started_from_outside() {
    let w = world();
    let id = w.add("Work");
    let kept = sw::store::files::data_dir().join("sandbox.json");
    let jobs = || sw::runner::JOBS.with(|j| j.borrow().clone());
    // macOS's own check is looked up, not linked: it builds everywhere, and finds no sandbox where there is no such check.
    let os = sw::plan::sandbox::inside::os_says();
    if !cfg!(target_os = "macos") { assert!(!os); }
    // The plan writer's turn-end hook runs in its mount, with the mark its program was started with.
    unsafe { std::env::set_var(sw::plan::sandbox::inside::MARK, "1") };
    assert!(sw::plan::sandbox::inside::inside());
    let _ = sw::plan::sandbox::tool();
    assert!(!kept.exists(), "what a probe from inside found (macOS: no sandbox inside another) says nothing of outside");
    assert_eq!(sw::runner::spawn_runner("finish", &id, &[]), 0);
    sw::runner::outside::start_asked(); // a pass inside starts nothing
    assert!(jobs().is_empty(), "no runner was started in the mount");
    assert_eq!(sw::runner::outside::asked(), vec![("finish".to_string(), id.clone(), vec![])]);
    // Outside (the keeper's housekeeping), it is started, once.
    unsafe { std::env::remove_var(sw::plan::sandbox::inside::MARK) };
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    assert_eq!(jobs().iter().filter(|j| j.0 == "finish").cloned().collect::<Vec<_>>(), vec![("finish".to_string(), id.clone(), vec![])]);
    assert!(sw::runner::outside::asked().is_empty());
}

/// Runs `sh -c <script>` in this platform's mount, guarding `guard`: (it worked, what it said).
fn in_mount(tool: Tool, guard: &[PathBuf], script: &str) -> (bool, String) {
    let words = tool.wrap(guard);
    let o = Command::new(&words[0]).args(&words[1..]).args(["sh", "-c", script]).output().unwrap();
    (o.status.success(), String::from_utf8_lossy(&o.stderr).into_owned())
}

#[test]
fn real_programs_a_write_into_the_guarded_folders_is_refused_and_one_elsewhere_works() {
    let w = world();
    let tool = Tool::native();
    if let Some(why) = tool.why_not(&w.base) {
        assert!(!cfg!(target_os = "macos"), "sandbox-exec must work on macOS: {why}");
        eprintln!("skipped: {why}");
        return;
    }
    let (tree, git, elsewhere) = (w.base.join("tree"), w.base.join("git"), w.base.join("elsewhere"));
    for d in [&tree, &git, &elsewhere] { std::fs::create_dir_all(d).unwrap(); }
    std::fs::write(tree.join("a.txt"), "a\n").unwrap();
    let guard = [tree.clone(), git.clone()];
    let q = |p: &Path| sw::util::shq(&p.display().to_string());
    for script in [format!("echo x > {}", q(&tree.join("new.txt"))), format!("echo x >> {}", q(&tree.join("a.txt"))), format!("rm {}", q(&tree.join("a.txt"))),
        format!("chmod u-w {}", q(&tree.join("a.txt"))), format!("mkdir {}", q(&git.join("refs"))), format!("echo x > {}", q(&git.join("index")))] {
        let (ok, said) = in_mount(tool, &guard, &script);
        assert!(!ok && said.contains(tool.denied()), "{script}: {said}");
    }
    assert_eq!(std::fs::read_to_string(tree.join("a.txt")).unwrap(), "a\n");
    assert!(!tree.join("new.txt").exists() && !git.join("refs").exists());
    let (ok, said) = in_mount(tool, &guard, &format!("echo x > {} && cat {}", q(&elsewhere.join("plan.md")), q(&tree.join("a.txt"))));
    assert!(ok, "{said}");
    assert_eq!(std::fs::read_to_string(elsewhere.join("plan.md")).unwrap(), "x\n");
    if tool == Tool::SandboxExec {
        // Why a program's own sandbox is turned off inside ours: macOS allows no sandbox inside another.
        let (ok, said) = in_mount(tool, &guard, "sandbox-exec -p '(version 1)(allow default)' true");
        eprintln!("a sandbox inside ours: {}", if ok { "allowed".to_string() } else { format!("refused: {}", said.trim()) });
    }
}
