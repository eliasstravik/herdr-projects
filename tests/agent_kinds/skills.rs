//! The skills and commands a program takes in its chat, read where it keeps them (agents/skills.rs), as the herdr web
//! UI's chat box lists them: Claude Code's as `/name` (yours, the project's, its commands, its plugins' turned on as
//! `/plugin:name`), Codex's skills as `$name` and its prompts as `/prompts:name`, pi's skills as `/skill:name`; what
//! each does in its first sentence; and the ones that match a word typed, best first.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::agents::skills::{matching, offered, triggers, Skill};

fn write(p: &Path, text: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }
fn typed(l: &[Skill]) -> Vec<&str> { l.iter().map(|s| s.typed.as_str()).collect() }
fn refs<'a>(l: &[&'a Skill]) -> Vec<&'a str> { l.iter().map(|s| s.typed.as_str()).collect() }
fn about<'a>(l: &'a [Skill], t: &str) -> &'a str { &l.iter().find(|s| s.typed == t).unwrap().about }

#[test]
fn claude_code_offers_your_skills_the_projects_the_commands_and_the_plugins_turned_on() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let claude = w.base.join("claude");
    write(&claude.join("skills/review-pr/SKILL.md"), "---\nname: review-pr\ndescription: Review a pull request. Use it when the human asks for a review, e.g. of a branch.\n---\nThe body.\n");
    write(&claude.join("skills/hidden/SKILL.md"), "---\nname: hidden\ndescription: Only for Claude itself\nuser-invocable: false\n---\n");
    write(&claude.join("skills/plain/SKILL.md"), "# Plain things\n\nDoes plain things.\n");
    write(&w.root.join(".claude/skills/ship/SKILL.md"), "---\nname: ship\ndescription: >\n  Ship the\n  release.\n---\n");
    write(&claude.join("commands/git/commit.md"), "---\ndescription: \"Commit what is staged\"\n---\n");
    write(&w.root.join(".claude/commands/deploy.md"), "Deploy the app.\n");
    let plug = w.base.join("plugin");
    write(&plug.join("skills/lint/SKILL.md"), "---\nname: lint\ndescription: Lint the code\n---\n");
    write(&w.base.join("off/skills/nope/SKILL.md"), "---\nname: nope\ndescription: Off\n---\n");
    write(&claude.join("settings.json"), &json!({ "enabledPlugins": { "tools@market": true, "off@market": false } }).to_string());
    write(&claude.join("plugins/installed_plugins.json"), &json!({ "plugins": { "tools@market": [{ "installPath": plug }], "off@market": [{ "installPath": w.base.join("off") }] } }).to_string());
    // from a folder inside the repo: the project's are those of the repo's folders up to its top
    let l = offered("claude", &w.root.join("src"));
    assert_eq!(typed(&l), vec!["/deploy", "/git:commit", "/plain", "/review-pr", "/ship", "/tools:lint"]);
    assert_eq!((about(&l, "/review-pr"), about(&l, "/ship"), about(&l, "/plain")), ("Review a pull request", "Ship the release", "Plain things"));
    assert_eq!((about(&l, "/git:commit"), about(&l, "/deploy")), ("Commit what is staged", "Deploy the app"));
    assert_eq!(triggers("claude"), "/");
    // what matches a word typed: those that start with it, then a word of theirs, then the rest that hold it
    assert_eq!(refs(&matching(&l, "/")).len(), 6);
    assert_eq!(refs(&matching(&l, "/rev")), vec!["/review-pr"]);
    assert_eq!(refs(&matching(&l, "/commit")), vec!["/git:commit"], "a namespace is left out");
    assert_eq!(refs(&matching(&l, "/pr")), vec!["/review-pr"]);
    assert_eq!(refs(&matching(&l, "/s")), vec!["/ship", "/tools:lint"], "the one that starts with it first");
    assert!(matching(&l, "$rev").is_empty(), "another trigger matches nothing");
}

#[test]
fn codex_offers_skills_with_a_dollar_and_its_prompts_and_pi_its_skills_as_skill_name() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let codex = w.base.join("codex");
    write(&codex.join("skills/review/SKILL.md"), "---\nname: review\ndescription: Review changes\n---\n");
    write(&codex.join("skills/bare/SKILL.md"), "---\nname: bare\n---\nNo description: Codex leaves it out.\n");
    write(&codex.join("skills/.system/imagegen/SKILL.md"), "---\nname: imagegen\ndescription: Make a picture\n---\n");
    write(&w.base.join("home/.agents/skills/notes/SKILL.md"), "---\nname: notes\ndescription: Take notes\n---\n");
    write(&w.root.join(".agents/skills/release/SKILL.md"), "---\nname: release\ndescription: Cut a release\n---\n");
    write(&codex.join("prompts/fix.md"), "Fix the bug.\n");
    let l = offered("codex", &w.root);
    assert_eq!(typed(&l), vec!["$imagegen", "$notes", "$release", "$review", "/prompts:fix"]);
    assert_eq!(triggers("codex"), "$/");
    assert_eq!(refs(&matching(&l, "$re")), vec!["$release", "$review"]);
    assert_eq!(refs(&matching(&l, "/f")), vec!["/prompts:fix"]);

    let pi = w.base.join("pi");
    write(&pi.join("skills/group/deep/SKILL.md"), "---\nname: deep\ndescription: Found deep down\n---\n");
    write(&pi.join("skills/loose.md"), "---\nname: loose\ndescription: A loose one\n---\n");
    write(&w.root.join(".pi/skills/proj/SKILL.md"), "---\nname: proj\ndescription: The project's\n---\n");
    write(&pi.join("prompts/plan.md"), "Plan it\n");
    let l = offered("pi", &w.root);
    // ~/.agents/skills and the project's .agents/skills are pi's too
    assert_eq!(typed(&l), vec!["/plan", "/skill:deep", "/skill:loose", "/skill:notes", "/skill:proj", "/skill:release"]);
    assert_eq!(refs(&matching(&l, "/de")), vec!["/skill:deep"]);
    // a program whose skills the web UI does not know either offers none: what is typed goes to it as it is
    assert!(offered("kimi", &w.root).is_empty() && triggers("kimi").is_empty());
}

#[test]
fn codex_offers_the_skills_of_the_plugins_turned_on_in_its_config_and_leaves_out_a_skill_turned_off() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let codex = w.base.join("codex");
    write(&codex.join("skills/local-check/SKILL.md"), "---\nname: local-check\ndescription: Check here\n---\n");
    write(&codex.join("skills/turned-off/SKILL.md"), "---\nname: turned-off\ndescription: Off in config\n---\n");
    write(&codex.join("plugins/cache/demo/tools/1.0.0/skills/plugin-check/SKILL.md"), "---\nname: plugin-check\ndescription: Check from a plugin\n---\n");
    write(&codex.join("plugins/cache/demo/other/1.0.0/skills/not-on/SKILL.md"), "---\nname: not-on\ndescription: A plugin not turned on\n---\n");
    // no [plugins] table: every plugin in the cache counts
    assert_eq!(typed(&offered("codex", &w.root)), vec!["$local-check", "$not-on", "$plugin-check", "$turned-off"]);
    write(&w.base.join("codex.toml"), "");
    write(&codex.join("config.toml"), &format!("[plugins.\"tools@demo\"]\nenabled = true\n\n[plugins.\"other@demo\"]\nenabled = false\n\n[[skills.config]]\npath = \"{}\"\nenabled = false\n", codex.join("skills/turned-off/SKILL.md").display()));
    let l = offered("codex", &w.root);
    assert_eq!(typed(&l), vec!["$local-check", "$plugin-check"]);
    assert_eq!(about(&l, "$plugin-check"), "Check from a plugin");
}
