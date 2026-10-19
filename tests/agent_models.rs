//! Model names (HP-16): a model a program does not take is refused when a setting names it, with the names it does
//! take, read from the program's own list (codex's for its sign-in, pi, cursor, opencode) or its own rule (claude); a
//! program with neither, or pointed at a provider of its own, is not checked; `herdr-projects models` lists them.
use crate::common;
use common::*;
use serde_json::json;
use sw::agents::kinds::Read;
use sw::agents::models::listed::read;
use sw::agents::models::say::models_command;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::{setting_text, Scope};

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn refused(name: &str, value: &str, re: &str) { let e = set_value(name, &json!(value), &Scope::you()).unwrap_err(); assert!(has(&e.msg, re), "{name} {value}: {re}: {}", e.msg); }
fn taken(name: &str, value: &str) { set_value(name, &json!(value), &Scope::you()).unwrap_or_else(|e| panic!("{name} {value}: {}", e.msg)); assert_eq!(setting_text(name, None), value); }
/// The lists the programs gave, as the board keeps them (the tests never run a real program for them).
fn lists(v: serde_json::Value) {
    let now = sw::clock::now();
    let mut o = serde_json::Map::new();
    for (k, l) in v.as_object().unwrap() { o.insert(k.clone(), json!({ "at": now, "taken": l["taken"], "shown": l["shown"] })); }
    std::fs::write(sw::store::files::data_dir().join("models.json"), serde_json::Value::Object(o).to_string()).unwrap();
}
const CODEX: &str = r#"{"models":[{"slug":"gpt-6.1-sol","visibility":"list"},{"slug":"gpt-6-luna","visibility":"list"},{"slug":"codex-auto-review","visibility":"hide"}]}"#;
const PI: &str = "provider   model                      context  max-out  thinking  images\nanthropic  claude-opus-5-5            1M       128K     yes       yes\nfireworks  accounts/fw/models/glm-5p2  1.0M     16.4K    yes       no\n";

#[test]
fn each_program_s_own_list_is_read_in_its_own_format() {
    let c = read(Read::Catalog, CODEX);
    assert_eq!(c.taken, ["gpt-6.1-sol", "gpt-6-luna", "codex-auto-review"]);
    assert_eq!(c.shown, ["gpt-6.1-sol", "gpt-6-luna"], "a hidden one is taken, not shown");
    // pi: its provider first; without it only where the name cannot be misread as a provider.
    let p = read(Read::Table, PI);
    assert_eq!(p.taken, ["anthropic/claude-opus-5-5", "claude-opus-5-5", "fireworks/accounts/fw/models/glm-5p2"]);
    assert_eq!(p.shown, ["anthropic/claude-opus-5-5", "fireworks/accounts/fw/models/glm-5p2"]);
    let cur = read(Read::Dashed, "Available models\n\nauto - Auto (default)\nclaude-opus-4-8-high - Claude Opus 4.8 1M\n\nTip: use --model <id> - to switch.\n");
    assert_eq!(cur.taken, ["auto", "claude-opus-4-8-high"]);
    let oc = read(Read::Lines, "[opencode-exe-dev] a note\nopencode/claude-opus-5-5\nanthropic/claude-opus-5-5\n");
    assert_eq!(oc.taken, ["opencode/claude-opus-5-5", "anthropic/claude-opus-5-5"]);
}

#[test]
fn a_model_the_program_does_not_take_is_refused_with_the_ones_it_does() {
    let _w = world();
    lists(json!({
        "codex": { "taken": ["gpt-6.1-sol", "gpt-6-luna", "codex-auto-review"], "shown": ["gpt-6.1-sol", "gpt-6-luna"] },
        "pi": { "taken": ["anthropic/claude-opus-5-5", "claude-opus-5-5", "fireworks/accounts/fw/models/glm-5p2"], "shown": ["anthropic/claude-opus-5-5", "fireworks/accounts/fw/models/glm-5p2"] },
        "cursor": { "taken": ["auto", "claude-opus-4-8-high", "claude-opus-4-8-low"], "shown": ["auto", "claude-opus-4-8-high", "claude-opus-4-8-low"] },
    }));
    // The user's report: a Codex model written the way another program writes it.
    refused("worker.agent", "codex/gpt-5.5", r#"^worker\.agent: codex has no model "gpt-5\.5"\. It takes gpt-6\.1-sol, gpt-6-luna \(from `codex debug models`\)\. Write it after the program: codex/gpt-6\.1-sol\.$"#);
    refused("worker.agent", "codex/gpt-6.1-sl", r"Did you mean gpt-6\.1-sol\?");
    taken("worker.agent", "codex/gpt-6.1-sol");
    taken("worker.agent", "codex/codex-auto-review");
    // pi: its provider first, or a name that cannot be misread; a thinking level after it.
    taken("reviewer.agent", "pi/anthropic/claude-opus-5-5");
    taken("reviewer.agent", "pi/claude-opus-5-5:high");
    taken("reviewer.agent", "pi/fireworks/accounts/fw/models/glm-5p2");
    refused("reviewer.agent", "pi/accounts/fw/models/glm-5p2", r"Did you mean fireworks/accounts/fw/models/glm-5p2\?");
    refused("reviewer.agent", "pi/openai/claude-opus-5-5", r"Did you mean anthropic/claude-opus-5-5\?");
    // cursor: a listed name, or a model whose variants are listed with its parameters in brackets.
    taken("worker.agent", "cursor/claude-opus-4-8-high");
    taken("worker.agent", "cursor/claude-opus-4-8[effort=high]");
    refused("worker.agent", "cursor/claude-opus-4-8", r"only with its parameters in brackets, as in cursor/claude-opus-4-8\[effort=high\]");
    refused("worker.agent", "cursor/gpt-9[effort=high]", r#"cursor has no model "gpt-9\[effort=high\]""#);
    // Every setting that names an agent, and a choice for one Coordinator only.
    refused("worker.fallback", "claude, codex/gpt-5.5", r"^worker\.fallback: codex has no model");
    refused("attempts.by", "codex/gpt-5.5, pi", r"^attempts\.by: codex has no model");
    refused("watcher.agent", "codex/o3", r"^watcher\.agent: codex has no headless one-shot mode, so it cannot be the Watcher");
    assert!(sw::coordinator::who::pick_of("codex/gpt-5.5", "").unwrap_err().msg.contains("codex has no model \"gpt-5.5\""));
    // A refused value changed nothing.
    assert_eq!(setting_text("worker.agent", None), "cursor/claude-opus-4-8[effort=high]");
    // Only a value being set is checked: one written in the file by hand is read as it is (reading never waits for
    // a program's list), and a start that fails on it says so.
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "worker.agent": "codex/gpt-5.5" }).to_string()).unwrap();
    assert_eq!(setting_text("worker.agent", None), "codex/gpt-5.5");
}

#[test]
fn claude_s_names_follow_its_own_rule_and_a_program_without_a_list_is_not_checked() {
    let _w = world();
    for m in ["opus", "sonnet", "haiku", "fable", "opusplan", "default", "opus[1m]", "claude-opus-5-5", "claude-opus-5-5[1m]", "claude-sonnet-4-5-20250929"] { taken("worker.agent", &format!("claude/{m}")); }
    refused("worker.agent", "claude/gpt-5", r#"^worker\.agent: claude has no model "gpt-5"\. It takes .*opus.*claude-opus-5-5.*, and \d+ more: `herdr-projects models claude` lists them all \(its short names, each the newest of its kind, and its full names\)\."#);
    // Invented and mistyped names (the review of HP-16): refused, with the nearest real one.
    refused("worker.agent", "claude/claude-opus-this-model-does-not-exist", r#"claude has no model "claude-opus-this-model-does-not-exist"\. It takes claude-opus-5-5, claude-opus-5, .*Write it after the program: claude/claude-opus-5-5\.$"#);
    refused("worker.agent", "claude/typo-claude-anything", r"claude has no model");
    refused("worker.agent", "claude/claude-opsu-5-5", r"Did you mean claude-opus-5-5\?");
    refused("worker.agent", "claude/claude-opus-5-5[2m]", r"claude has no model");
    refused("coordinator.agent", "claude/claude-", r"claude has no model");
    // Pointed at a provider of its own (a gateway, a cloud), its names are that provider's: not checked.
    for env in ["ANTHROPIC_BASE_URL", "CLAUDE_CODE_USE_BEDROCK"] {
        unsafe { std::env::set_var(env, "1") };
        let r = set_value("worker.agent", &json!("claude/us.anthropic.claude-opus-5-5-v1:0"), &Scope::you());
        unsafe { std::env::remove_var(env) };
        assert!(r.is_ok(), "{env}: {:?}", r.err().map(|e| e.msg));
    }
    // No list of its own: any name, as before.
    taken("worker.agent", "agy/gemini-3-pro");
    taken("worker.agent", "copilot/whatever-it-takes");
    // A list that cannot be had (not installed, not logged in): not checked, never a refusal.
    taken("worker.agent", "codex/gpt-5.5");
    // Codex on a provider of its own: its catalog says nothing of that provider's names.
    lists(json!({ "codex": { "taken": ["gpt-6.1-sol"], "shown": ["gpt-6.1-sol"] } }));
    refused("worker.agent", "codex/llama3", r"codex has no model");
    std::fs::write(std::env::var("HERDR_PROJECTS_CODEX_TOML").unwrap(), "model_provider = \"ollama\"\n\n[model_providers.ollama]\nname = \"Ollama\"\n").unwrap();
    taken("worker.agent", "codex/llama3");
}

#[test]
fn the_models_command_names_what_each_program_takes_as_a_setting_writes_it() {
    let _w = world();
    lists(json!({ "codex": { "taken": ["gpt-6.1-sol", "gpt-6-luna", "codex-auto-review"], "shown": ["gpt-6.1-sol", "gpt-6-luna"] } }));
    let (v, text) = models_command("codex").unwrap();
    assert_eq!(v["models"], json!(["gpt-6.1-sol", "gpt-6-luna"]));
    assert!(text.contains("worker.agent codex/gpt-6.1-sol:\n\ncodex/gpt-6.1-sol\ncodex/gpt-6-luna\n\nEffort"), "{text}");
    let (_, all) = models_command("").unwrap();
    assert!(has(&all, r"(?m)^codex +codex/gpt-6\.1-sol, codex/gpt-6-luna$"), "{all}");
    assert!(has(&all, r"(?m)^claude +\d+ names, such as claude/default, claude/fable, claude/opus, claude/sonnet: herdr-projects models claude lists them all$"), "{all}");
    assert!(models_command("claude").unwrap().1.contains("\nclaude/claude-opus-5-5\n"));
    assert!(has(&all, r"(?m)^droid +droid takes no model when the board starts it"), "{all}");
    assert!(has(&all, r"(?m)^agy +any name agy takes \(not checked"), "{all}");
    assert!(models_command("codx").unwrap_err().msg.contains("Did you mean codex?"));
}

#[test]
fn real_programs_the_list_is_asked_of_the_program_and_kept() {
    let w = world();
    let bin = w.base.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let count = w.base.join("asked");
    // Its app-server answers for a ChatGPT sign-in, as `codex app-server` does: JSON-RPC, one line each way.
    let page = r#"{"id":1,"result":{"data":[{"id":"gpt-6.1-sol","model":"gpt-6.1-sol","hidden":false},{"id":"gpt-6-luna","model":"gpt-6-luna","hidden":false},{"id":"codex-auto-review","model":"codex-auto-review","hidden":true}],"nextCursor":null}}"#;
    let account = r#"{"id":2,"result":{"account":{"type":"chatgpt","email":"x@y","planType":"pro"},"requiresOpenaiAuth":true}}"#;
    std::fs::write(bin.join("codex"), format!("#!/bin/sh\necho \"$1\" >> {}\nif [ \"$1\" = app-server ]; then\n  while read l; do case \"$l\" in *'\"model/list\"'*) printf '%s\\n' '{page}';; *'\"account/read\"'*) printf '%s\\n' '{account}';; esac; done\n  exit 0\nfi\nprintf '%s' '{CODEX}'\n", count.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(bin.join("codex"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let run = |args: &[&str]| { let o = std::process::Command::new(env!("CARGO_BIN_EXE_herdr-projects")).args(args).current_dir(&w.root)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display())).env_remove("HERDR_PROJECTS_NO_MODEL_LIST").env("HERDR_SOCKET_PATH", "/nonexistent").env("HERDR_BIN_PATH", "/bin/false").output().unwrap();
        (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned()) };
    let asked = || std::fs::read_to_string(&count).unwrap_or_default().lines().count();
    let (code, _, err) = run(&["set", "worker.agent", "codex/gpt-5.5"]);
    assert_eq!(code, 1);
    assert!(err.contains("codex has no model \"gpt-5.5\" for your ChatGPT sign-in (pro plan). It takes gpt-6.1-sol, gpt-6-luna (codex's own list for that sign-in)"), "{err}");
    assert_eq!(asked(), 1, "asked once: the list it gave is under a minute old");
    assert_eq!(std::fs::read_to_string(&count).unwrap(), "app-server\n", "its app-server, for the account it is signed in with");
    let (code, out, err) = run(&["set", "worker.agent", "codex/gpt-6-luna"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("codex/gpt-6-luna"));
    assert_eq!(asked(), 1, "kept: not asked again");
    let (_, out, _) = run(&["models", "codex"]);
    assert!(out.contains("codex/gpt-6.1-sol\ncodex/gpt-6-luna"), "{out}");
    assert!(out.contains("(here: your ChatGPT sign-in (pro plan))"), "{out}");
    // An app-server that gives no list (an older codex): its list command instead, with no sign-in to name.
    std::fs::remove_file(sw::store::files::data_dir().join("models.json")).unwrap();
    std::fs::write(bin.join("codex"), format!("#!/bin/sh\necho \"$1\" >> {}\n[ \"$1\" = app-server ] && exit 1\nprintf '%s' '{CODEX}'\n", count.display())).unwrap();
    let (code, _, err) = run(&["set", "worker.agent", "codex/gpt-5.5"]);
    assert_eq!(code, 1);
    assert!(err.contains("codex has no model \"gpt-5.5\". It takes gpt-6.1-sol, gpt-6-luna (from `codex debug models`)"), "{err}");
    // Installed, but its list cannot be read (not logged in, offline, broken): refused, with why, not let through
    // unchecked; and not kept, so the next try asks again.
    std::fs::remove_file(sw::store::files::data_dir().join("models.json")).unwrap();
    std::fs::write(bin.join("codex"), "#!/bin/sh\necho 'error: not logged in' >&2\nexit 2\n").unwrap();
    let (code, _, err) = run(&["set", "worker.agent", "codex/this-model-does-not-exist"]);
    assert_eq!(code, 1);
    assert!(err.contains("codex's list of models could not be read, so \"this-model-does-not-exist\" cannot be checked: `codex debug models` failed (exit 2): error: not logged in. Run `codex debug models` in a terminal here"), "{err}");
    assert!(!sw::store::files::data_dir().join("models.json").exists(), "a failed list is not kept");
    let (code, out, _) = run(&["set", "worker.agent", "codex"]);
    assert_eq!(code, 0, "without a model there is nothing to check: {out}");
    let (_, out, _) = run(&["models"]);
    assert!(has(&out, r"(?m)^codex +its list could not be read: `codex debug models` failed \(exit 2\)"), "{out}");
    // Not installed here: not checked.
    std::fs::remove_file(bin.join("codex")).unwrap();
    let (code, _, err) = run(&["set", "worker.agent", "codex/gpt-5.5"]);
    assert_eq!(code, 0, "{err}");
}
