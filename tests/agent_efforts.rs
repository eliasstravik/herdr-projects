//! Effort levels (HP-31): the levels a program takes are read from the program itself (Codex's catalog, for each
//! model; the others' `--help`), not from a list written in the board that goes out of date; a level it does not take
//! is refused with the ones it does; where they cannot be read, the table's are used; `herdr-projects models` says them.
use crate::common;
use common::*;
use serde_json::json;
use sw::agents::efforts::{levels, read, why_not, Ask};
use sw::agents::kinds::EffortsRead;
use sw::agents::models::say::models_command;
use sw::agents::start::{start_args, StartOpts};
use sw::json::Js;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::{setting_text, Scope};

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn refused(name: &str, value: &str, re: &str) { let e = set_value(name, &json!(value), &Scope::you()).unwrap_err(); assert!(has(&e.msg, re), "{name} {value}: {re}: {}", e.msg); }
fn taken(name: &str, value: &str) { set_value(name, &json!(value), &Scope::you()).unwrap_or_else(|e| panic!("{name} {value}: {}", e.msg)); assert_eq!(setting_text(name, None), value); }
/// The levels the programs said, as the board keeps them (the tests never run a real program for them).
fn said(v: serde_json::Value) {
    let now = sw::clock::now();
    let mut o = serde_json::Map::new();
    for (k, l) in v.as_object().unwrap() { o.insert(k.clone(), json!({ "at": now, "levels": l["levels"], "by": l["by"] })); }
    std::fs::write(sw::store::files::data_dir().join("efforts.json"), serde_json::Value::Object(o).to_string()).unwrap();
}
fn lv(e: &[&str]) -> serde_json::Value { json!(e.iter().map(|x| json!({ "effort": x, "description": "…" })).collect::<Vec<_>>()) }
fn codex_catalog() -> String {
    let all = ["low", "medium", "high", "xhigh", "max", "ultra"];
    json!({ "models": [
        { "slug": "gpt-6.1-sol", "visibility": "list", "supported_reasoning_levels": lv(&all) },
        { "slug": "gpt-6-luna", "visibility": "list", "supported_reasoning_levels": lv(&all[..5]) },
        { "slug": "codex-auto-review", "visibility": "hide", "supported_reasoning_levels": lv(&all[..5]) },
    ] }).to_string()
}
const CLAUDE_HELP: &str = "Options:\n  --effort-mode <m>   not this one (a, b)\n  --effort <level>                      Effort level for the current session\n                                        (low, medium, high, xhigh, max)\n  --environment <id>  Create a new cloud session\n";
const PI_HELP: &str = "  --model <pattern>   Model pattern or ID (supports \"provider/id\" and optional \":<thinking>\")\n  --thinking <level>             Set thinking level: off, minimal, low, medium, high, xhigh, max\n  --extension, -e <path>  Load an extension\n";
const CODEX: &[&str] = &["low", "medium", "high", "xhigh", "max", "ultra"];
const LUNA: &[&str] = &["low", "medium", "high", "xhigh", "max"];

#[test]
fn each_program_says_its_levels_in_its_own_way() {
    let c = read(EffortsRead::Catalog, "", &codex_catalog());
    assert_eq!(c.levels, CODEX, "every level some model takes");
    assert_eq!(c.by[1], ("gpt-6-luna".to_string(), LUNA.iter().map(|s| s.to_string()).collect::<Vec<_>>()));
    assert_eq!(read(EffortsRead::Help, "--effort", CLAUDE_HELP).levels, ["low", "medium", "high", "xhigh", "max"], "on the lines its argument goes on to, not a longer argument's");
    assert_eq!(read(EffortsRead::Help, "--thinking", PI_HELP).levels, ["off", "minimal", "low", "medium", "high", "xhigh", "max"]);
    assert_eq!(read(EffortsRead::Help, "--reasoning-effort", "  -r, --reasoning-effort  Reasoning effort [string] [choices: \"none\", \"low\", \"high\"]\n").levels, ["none", "low", "high"]);
    assert_eq!(read(EffortsRead::Help, "--effort", "  --effort <level>  low|medium|high (default: high)\n").levels, ["low", "medium", "high"]);
    assert!(read(EffortsRead::Help, "--effort", "  --effort <level>  How hard it thinks.\n  --model <m>  (a, b)\n").levels.is_empty(), "a help that names none: the table's are used");
    assert!(read(EffortsRead::Catalog, "", "not json").levels.is_empty());
}

#[test]
fn a_level_the_program_does_not_take_is_refused_with_the_ones_it_does() {
    let _w = world();
    said(json!({
        "codex": { "levels": CODEX, "by": { "gpt-6.1-sol": CODEX, "gpt-6-luna": LUNA } },
        "claude": { "levels": ["low", "medium", "high", "xhigh", "max"], "by": {} },
    }));
    // The report: max (and ultra) are Codex's own levels now, and minimal is gone.
    taken("worker.agent", "codex");
    taken("worker.effort", "max");
    taken("worker.effort", "ultra");
    refused("worker.effort", "minimal", r#"^worker\.effort: codex has no effort "minimal": it takes low, medium, high, xhigh, max, ultra, as `codex debug models` lists them \(or default, for its own\)\.$"#);
    refused("worker.effort", "maxx", r"Did you mean max\?");
    // Each model its own: gpt-6-luna has no ultra.
    taken("worker.agent", "codex/gpt-6-luna");
    refused("worker.effort", "ultra", r#"^worker\.effort: codex's gpt-6-luna has no effort "ultra": it takes low, medium, high, xhigh, max, as `codex debug models` lists them"#);
    taken("worker.effort", "max");
    // A model it does not say levels for has every level any model takes.
    assert_eq!(levels("codex", "gpt-7", Ask::Kept).unwrap().levels, CODEX);
    // Claude Code's, from its --help; a program that takes no effort at start, with why.
    taken("reviewer.agent", "claude");
    refused("reviewer.effort", "ultra", r#"claude has no effort "ultra": it takes low, medium, high, xhigh, max, as `claude --help` lists them"#);
    taken("reviewer.agent", "cursor");
    refused("reviewer.effort", "high", r"cursor takes its effort in the model's name: cursor/claude-opus-4-8\[effort=high\]: leave it at default\.");
    taken("reviewer.agent", "opencode");
    refused("reviewer.effort", "high", r"opencode takes no effort level when the board starts it");
    // The Coordinator for one use: its effort is checked against the model it is given.
    assert!(sw::coordinator::who::pick_of("codex/gpt-6-luna", "ultra").unwrap_err().msg.contains("codex's gpt-6-luna has no effort \"ultra\""));
    assert_eq!(sw::coordinator::who::pick_of("codex/gpt-6.1-sol", "ultra").unwrap().unwrap().s("effort"), "ultra");
}

#[test]
fn where_the_levels_cannot_be_read_the_table_s_are_used() {
    let _w = world();
    // Nothing said (not installed, offline): the levels in the table of programs, never a refusal of every level.
    assert_eq!(levels("codex", "", Ask::Fresh).unwrap().levels, CODEX);
    assert_eq!(levels("codex", "", Ask::Fresh).unwrap().from, "");
    taken("worker.agent", "codex");
    taken("worker.effort", "max");
    refused("worker.effort", "banana", r#"^worker\.effort: codex has no effort "banana": it takes low, medium, high, xhigh, max, ultra \(or default, for its own\)\.$"#);
    taken("worker.agent", "pi");
    taken("worker.effort", "max");
    taken("worker.agent", "agy");
    refused("worker.effort", "minimal", r#"agy has no effort "minimal": it takes low, medium, high, xhigh, max"#);
    // A program whose table names no levels takes any.
    assert_eq!(why_not("grok", "", "whatever", Ask::Fresh), None);
}

#[test]
fn a_start_and_a_read_go_by_the_levels_said_last_and_never_wait() {
    let _w = world();
    said(json!({ "codex": { "levels": CODEX, "by": { "gpt-6-luna": LUNA } } }));
    let args = |model: &str, effort: &str| start_args("codex", &StartOpts { model: model.into(), effort: effort.into(), ..Default::default() }).unwrap().join(" ");
    assert!(args("", "ultra").contains("model_reasoning_effort=\"ultra\""));
    assert!(!args("gpt-6-luna", "ultra").contains("model_reasoning_effort"), "a level its model does not take is left out");
    assert!(args("gpt-6-luna", "max").contains("model_reasoning_effort=\"max\""));
    // A level written in the file by hand that its program does not take: the next place down is read, and it is said.
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "worker.agent": "codex/gpt-6-luna", "worker.effort": "ultra" }).to_string()).unwrap();
    assert_eq!(setting_text("worker.effort", None), "default");
    assert!(sw::store::settings::show::problems(None).join("\n").contains("worker.effort: codex's gpt-6-luna has no effort \"ultra\""));
}

#[test]
fn the_models_command_says_each_program_s_levels() {
    let _w = world();
    said(json!({ "codex": { "levels": CODEX, "by": { "gpt-6.1-sol": CODEX, "gpt-6-luna": LUNA, "codex-auto-review": LUNA } } }));
    let (v, text) = models_command("codex").unwrap();
    assert_eq!(v["efforts"]["levels"], json!(CODEX));
    assert_eq!(v["efforts"]["by"], json!({ "gpt-6-luna": LUNA, "codex-auto-review": LUNA }), "only the models whose own levels differ");
    assert!(text.ends_with("\n\nEffort (worker.effort and the like): low, medium, high, xhigh, max, ultra (from `codex debug models`); gpt-6-luna, codex-auto-review only low, medium, high, xhigh, max."), "{text}");
    let (_, all) = models_command("").unwrap();
    assert!(has(&all, r"(?m)^codex .*\n +effort: low, medium, high, xhigh, max, ultra \(from `codex debug models`\)"), "{all}");
    assert!(has(&all, r"(?m)^cursor .*\n +effort: cursor takes its effort in the model's name"), "{all}");
    assert!(has(&all, r"(?m)^droid .*\n +effort: droid takes no effort level when the board starts it$"), "{all}");
    assert!(has(&all, r"(?m)^grok .*\n +effort: any level$"), "{all}");
}

#[test]
fn real_programs_the_levels_are_asked_of_the_program_and_kept() {
    let w = world();
    let bin = w.base.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let count = w.base.join("asked");
    std::fs::write(bin.join("codex"), format!("#!/bin/sh\necho x >> {}\nprintf '%s' '{}'\n", count.display(), codex_catalog())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(bin.join("codex"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let run = |args: &[&str]| { let o = std::process::Command::new(env!("CARGO_BIN_EXE_herdr-projects")).args(args).current_dir(&w.root)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display())).env_remove("HERDR_PROJECTS_NO_MODEL_LIST").env("HERDR_SOCKET_PATH", "/nonexistent").env("HERDR_BIN_PATH", "/bin/false").output().unwrap();
        (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned()) };
    let asked = || std::fs::read_to_string(&count).unwrap_or_default().lines().count();
    let (code, _, err) = run(&["set", "worker.agent", "codex"]);
    assert_eq!(code, 0, "{err}");
    let before = asked();
    let (code, _, err) = run(&["set", "worker.effort", "max"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(asked(), before + 1, "its levels asked once");
    let (code, _, err) = run(&["set", "worker.effort", "minimal"]);
    assert_eq!(code, 1);
    assert!(err.contains("codex has no effort \"minimal\": it takes low, medium, high, xhigh, max, ultra, as `codex debug models` lists them"), "{err}");
    assert_eq!(asked(), before + 1, "kept: a list under a minute old is not asked again");
    // Installed, but it says nothing (not logged in): the table's levels, never a refusal of every level.
    std::fs::remove_file(sw::store::files::data_dir().join("efforts.json")).unwrap();
    std::fs::write(bin.join("codex"), "#!/bin/sh\necho 'error: not logged in' >&2\nexit 2\n").unwrap();
    let (code, _, err) = run(&["set", "worker.effort", "high"]);
    assert_eq!(code, 0, "{err}");
    assert!(!sw::store::files::data_dir().join("efforts.json").exists(), "nothing said, nothing kept");
}
