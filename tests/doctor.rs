//! `herdr-projects doctor` (HP-32), on what each area found: the program and its launcher, herdr's plugin, the agent
//! programs, other machines and the disks; each problem one line with what to do, and the count at the end.
use crate::common;
use common::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use sw::doctor::{agents, disk, machines, plugin, program, problems, say, Found};

fn program_file(p: &Path) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, "#!/bin/sh\n").unwrap(); use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap(); }
fn launcher_for(p: &Path) -> String { format!("#!/bin/sh\nexport HERDR_PROJECTS_DATA_DIR='/d'\nexec {} \"$@\"\n", sw::util::shq(&p.display().to_string())) }
/// A plugin folder with its downloaded program, the launcher beside the data, and a PATH folder.
struct Setup { root: PathBuf, prog: PathBuf, launcher: PathBuf, path: PathBuf }
fn setup(w: &World) -> Setup {
    let root = w.base.join("plugin");
    let prog = root.join("bin").join(sw::install::asset_name(sw::install::platform().unwrap()));
    program_file(&prog);
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"herdr-projects\"\nversion = \"1.1.0\"\n").unwrap();
    let launcher = w.base.join("data/bin/herdr-projects");
    let path = w.base.join("path");
    std::fs::create_dir_all(&path).unwrap();
    Setup { root, prog, launcher, path }
}
fn here(s: &Setup, text: Option<String>, root: bool) -> program::Here {
    program::Here { exe: s.prog.clone(), version: "1.1.0".into(), launcher: s.launcher.clone(), launcher_text: text, plugin_root: root.then(|| s.root.clone()), path: s.path.display().to_string() }
}
fn bad(f: &[Found]) -> Vec<String> { f.iter().filter(|f| f.problem).map(|f| format!("{} → {}", f.what, f.fix)).collect() }

#[test]
fn each_problem_is_one_line_with_what_to_do_and_the_count_ends_it() {
    let all = vec![Found::ok("herdr 0.9.3 runs the plugin"), Found::bad("codex is not signed in (worker.agent)", "Run `codex login` in a terminal here.")];
    assert_eq!(say(&all), "✓ herdr 0.9.3 runs the plugin\n✗ codex is not signed in (worker.agent)\n  → Run `codex login` in a terminal here.\n1 problem.");
    assert_eq!(problems(&all), 1);
    assert!(say(&[Found::ok("fine")]).ends_with("No problems."));
    // Older installs: a line that goes on from the one before is said with it; only what is left to do is a problem.
    let old = sw::doctor::old_installs(vec![("Old herdr-projects (0.2): its projects are in ~/x.".into(), false), ("  The old version finds them with HERDR_PROJECTS_ROOT=~/x.".into(), false), ("HERDR_PROJECTS_ROOT is set.".into(), true)]);
    assert_eq!(old.len(), 2);
    assert!(old[0].what.ends_with("projects are in ~/x. The old version finds them with HERDR_PROJECTS_ROOT=~/x."), "{:?}", old[0]);
    assert!(old[1].problem);
}

#[test]
fn the_launcher_must_name_the_plugin_s_own_program() {
    let w = world();
    let s = setup(&w);
    assert_eq!(program::launcher_target(&launcher_for(Path::new("/a/it's/herdr-projects"))), Some(PathBuf::from("/a/it's/herdr-projects")));
    // Missing: what to run names the plugin's own run.sh.
    let f = program::check(&here(&s, None, true));
    assert!(f[0].what.starts_with("herdr-projects 1.1.0, downloaded: "), "{:?}", f[0]);
    assert!(bad(&f)[0].contains("is missing") && bad(&f)[0].contains("/plugin/bin/run.sh install"), "{:?}", bad(&f));
    // A program that is not there.
    let gone = s.root.join("old/herdr-projects");
    assert!(bad(&program::check(&here(&s, Some(launcher_for(&gone)), true)))[0].contains("which is not there"));
    // Another program than the plugin's.
    let other = w.base.join("elsewhere/herdr-projects");
    program_file(&other);
    let f = bad(&program::check(&here(&s, Some(launcher_for(&other)), true)));
    assert!(f[0].contains("not the plugin's program in"), "{f:?}");
    // Without herdr's word on the plugin, any program that is there will do.
    assert!(!bad(&program::check(&here(&s, Some(launcher_for(&other)), false)))[0].contains("plugin's program"));
    // The plugin's own program, built in its folder, is right too.
    let built = s.root.join("target/release/herdr-projects");
    program_file(&built);
    assert!(!bad(&program::check(&here(&s, Some(launcher_for(&built)), true))).iter().any(|l| l.contains("launcher")));
}

#[test]
fn a_shell_must_find_the_launcher_on_the_path() {
    let w = world();
    let s = setup(&w);
    std::fs::create_dir_all(s.launcher.parent().unwrap()).unwrap();
    program_file(&s.launcher);
    std::fs::write(&s.launcher, launcher_for(&s.prog)).unwrap();
    let f = bad(&program::check(&here(&s, Some(launcher_for(&s.prog)), true)));
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].starts_with("herdr-projects is not on your PATH → To type herdr-projects in any shell"), "{f:?}");
    // A link to the launcher (~/.local/bin/herdr-projects) is the launcher.
    std::os::unix::fs::symlink(&s.launcher, s.path.join("herdr-projects")).unwrap();
    let f = program::check(&here(&s, Some(launcher_for(&s.prog)), true));
    assert_eq!(bad(&f), Vec::<String>::new());
    assert!(f[1].what.ends_with("and a shell finds it on the PATH"), "{:?}", f[1]);
    // A launcher that cannot be run: agents and hooks cannot use it, and a shell does not find it.
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&s.launcher, std::fs::Permissions::from_mode(0o644)).unwrap();
    let f = bad(&program::check(&here(&s, Some(launcher_for(&s.prog)), true)));
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].contains("cannot be run: it is not executable → Run: sh "), "{f:?}");
    std::fs::set_permissions(&s.launcher, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Something else of that name first.
    std::fs::remove_file(s.path.join("herdr-projects")).unwrap();
    program_file(&s.path.join("herdr-projects"));
    assert!(bad(&program::check(&here(&s, Some(launcher_for(&s.prog)), true)))[0].contains("comes first on your PATH"));
}

#[test]
fn a_program_older_than_its_plugin_is_to_be_updated() {
    let w = world();
    let s = setup(&w);
    std::fs::write(s.root.join("Cargo.toml"), "[package]\nversion = \"1.2.0\"\n").unwrap();
    let f = bad(&program::check(&here(&s, Some(launcher_for(&s.prog)), true)));
    assert!(f.iter().any(|l| l == "This program is 1.1.0, older than the plugin's 1.2.0 → Run: herdr-projects update 1.2.0"), "{f:?}");
}

fn ours(enabled: bool, root: &Path, local: bool) -> serde_json::Value {
    json!({ "plugin_id": "herdr-projects", "enabled": enabled, "plugin_root": root.display().to_string(), "version": "1.1.0", "min_herdr_version": "0.9.3", "source": { "kind": if local { "local" } else { "github" } } })
}
#[test]
fn herdr_must_run_the_plugin_enabled_from_a_folder_that_is_there() {
    let w = world();
    let s = setup(&w);
    std::fs::write(s.root.join("herdr-plugin.toml"), "id = \"herdr-projects\"\n").unwrap();
    let h = |v: Option<&str>, list: Vec<serde_json::Value>| plugin::Herdr { version: v.map(String::from), plugins: Ok(list) };
    assert!(bad(&plugin::check(&h(None, vec![])))[0].starts_with("herdr does not run here"));
    assert!(bad(&plugin::check(&h(Some("0.9.3"), vec![])))[0].ends_with("→ Run: herdr plugin install eliasstravik/herdr-projects"));
    assert!(bad(&plugin::check(&h(Some("0.9.3"), vec![ours(false, &s.root, true)])))[0].ends_with("but turned off → Run: herdr plugin enable herdr-projects"));
    assert!(bad(&plugin::check(&h(Some("0.9.3"), vec![ours(true, &w.base.join("gone"), true)])))[0].contains("which is gone"));
    assert!(bad(&plugin::check(&h(Some("0.9.1"), vec![ours(true, &s.root, true)])))[0].starts_with("herdr 0.9.1 is older than 0.9.3"));
    let f = plugin::check(&h(Some("0.9.4"), vec![ours(true, &s.root, true)]));
    assert_eq!(bad(&f), Vec::<String>::new());
    assert!(f[0].what.starts_with("herdr 0.9.4 runs the plugin 1.1.0, linked from "), "{:?}", f[0]);
    assert!(plugin::check(&h(Some("0.9.4"), vec![ours(true, &s.root, false)]))[0].what.ends_with("installed from GitHub"));
    assert_eq!(plugin::root_of(&h(Some("0.9.4"), vec![ours(true, &s.root, true)])), Some(s.root.clone()));
    assert!(bad(&plugin::check(&plugin::Herdr { version: Some("0.9.4".into()), plugins: Err("no server".into()) }))[0].contains("did not list its plugins: no server"));
}

#[test]
fn each_program_must_be_installed_new_enough_and_signed_in() {
    let _w = world();
    use agents::Sign;
    let p = |k: &str, found: bool, v: &str, sign: Sign| agents::Prog { kind: k.into(), named_by: vec!["worker.agent".into()], found, version: v.into(), sign, ..Default::default() };
    let f = agents::check(&[p("claude", true, "2.1.301", Sign::In), p("pi", true, "1.1.0", Sign::Unknown)]);
    assert_eq!(f.iter().map(|f| f.what.as_str()).collect::<Vec<_>>(), ["claude 2.1.301, signed in (worker.agent)", "pi 1.1.0 (worker.agent)"]);
    assert_eq!(bad(&agents::check(&[p("codex", false, "", Sign::Unknown)])), ["codex is not installed here (worker.agent) → Install codex, or choose another program: herdr-projects set worker.agent <program>"]);
    assert_eq!(bad(&agents::check(&[p("codex", true, "0.162.1", Sign::Out)])), ["codex is not signed in (worker.agent) → Run `codex login` in a terminal here."]);
    assert!(bad(&agents::check(&[p("claude", true, "2.1.100", Sign::In)]))[0].starts_with("claude 2.1.100 is older than 2.1.296"));
    let mut m = p("mastracode", true, "1.0.0", Sign::Unknown);
    m.integration = Some(false);
    assert!(bad(&agents::check(&[m]))[0].ends_with("→ Run: herdr integration install mastracode"));
    // A sign-in command that does not answer is no "signed in": said, with what to run.
    assert_eq!(bad(&agents::check(&[p("claude", true, "2.1.301", Sign::NoAnswer("it did not answer in 5s".into()))])), ["Could not tell whether claude is signed in: `claude auth status`: it did not answer in 5s → Run `claude auth status` in a terminal here to see why; if it says you are signed out, run `claude auth login`."]);
    // A program not installed for a repo's or a task's setting is set there.
    let mut q = p("codex", false, "", Sign::Unknown);
    q.named_by = vec!["reviewer.agent for proj".into()];
    assert!(bad(&agents::check(&[q]))[0].ends_with("herdr-projects set reviewer.agent <program> --repo proj"));
    // What a status command says: it failed, or it says it is signed out (cursor-agent exits 0 either way).
    assert!(agents::signed_out(1, "{\"loggedIn\": false}"));
    assert!(agents::signed_out(0, "Not logged in"));
    assert!(!agents::signed_out(0, "Logged in using ChatGPT"));
}

#[test]
fn a_version_command_that_hangs_is_said_and_never_asked_again_at_once() {
    let _w = world();
    // What the last ask found (installed.rs gives `--version` 5s, then writes this): it did not answer.
    let f = sw::store::files::data_dir().join("versions.json");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
    v["claude"] = json!({ "v": "", "at": sw::clock::now(), "found": true, "late": true });
    std::fs::write(&f, v.to_string()).unwrap();
    let p = agents::ask("claude", vec!["worker.agent".into()]);
    assert!(p.found && p.late && p.sign == agents::Sign::Unknown, "{p:?}");
    assert_eq!(bad(&agents::check(&[p])), ["claude did not say its version in time: `claude --version` hangs (worker.agent) → Run `claude --version` in a terminal here to see why, and fix or reinstall claude."]);
}

#[test]
fn other_machines_must_answer_and_be_able_to_ask_this_one() {
    let _w = world();
    assert_eq!(machines::check(None)[0], Found::ok("No other machines to reach"));
    let r = json!({ "here": { "label": "me", "here": true, "ok": false, "why": { "line": "a login shell does not find herdr-projects", "detail": ["Put it on the PATH in ~/.profile."] } },
        "machines": [
            { "label": "vm-a", "target": "a.ts.net", "ok": true, "version": "1.1.0", "tasks": 1 },
            { "label": "vm-b", "target": "b.ts.net", "ok": false, "why": { "line": "ssh could not connect", "detail": ["", "Check that it is on: ssh b.ts.net"] } },
            { "label": "vm-c", "target": "c.ts.net", "ok": true, "version": "", "tasks": 0 }] });
    let f = machines::check(Some(&r));
    assert_eq!(bad(&f), [
        "Other machines cannot ask this one: a login shell does not find herdr-projects → Put it on the PATH in ~/.profile.",
        "vm-b (b.ts.net) does not answer: ssh could not connect → Check that it is on: ssh b.ts.net",
        "vm-c answers, but runs an older herdr-projects → Run herdr-projects update on vm-c."]);
    assert!(f.contains(&Found::ok("vm-a answers: herdr-projects 1.1.0, 1 task")));
}

#[test]
fn a_disk_low_on_space_is_a_problem() {
    let _w = world();
    let gb = 1_000_000_000u64;
    let d = |free: u64| disk::Disk { mount: "/".into(), free, total: 100 * gb, holds: vec!["the board's data".into(), "worktrees".into()], waiting: vec![] };
    assert_eq!(disk::check(&[d(16 * gb)]), [Found::ok("16 GB free of 100 GB on / (the board's data, worktrees)")]);
    assert!(bad(&disk::check(&[d(3_200_000_000)]))[0].starts_with("Only 3.2 GB free on /, which holds the board's data, worktrees: new tasks may fail at their setup → Free some space"));
    assert_eq!(disk::gb(640_000_000), "640 MB");
    // This machine's own disks: the board's data is on one of them.
    let all = disk::ask();
    assert!(all.iter().any(|d| d.holds.iter().any(|h| h == "the board's data") && d.total > 0), "{all:?}");
}
