//! The skills of the other programs the herdr web UI lists them for, read as it reads them (agents/skills/): OpenCode,
//! Devin, Grok, Hermes, omp and Antigravity (agy), each from its own places and by its own rules.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::agents::skills::{offered, triggers, Skill};

fn write(p: &Path, text: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }
fn skill(dir: &Path, name: &str, about: &str) { write(&dir.join(name).join("SKILL.md"), &format!("---\nname: {name}\ndescription: {about}\n---\n")); }
fn typed(l: &[Skill]) -> Vec<&str> { l.iter().map(|s| s.typed.as_str()).collect() }
fn about<'a>(l: &'a [Skill], t: &str) -> &'a str { &l.iter().find(|s| s.typed == t).unwrap_or_else(|| panic!("no {t} in {l:?}")).about }

#[test]
fn opencode_offers_its_commands_its_own_skill_and_the_skills_of_its_folders_and_claude_codes() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let home = w.base.join("home");
    skill(&home.join(".config/opencode/skills"), "my-skill", "Mine. Use it often.");
    write(&home.join(".config/opencode/skills/wrong/SKILL.md"), "---\nname: not-its-folder\ndescription: Named unlike its folder\n---\n");
    write(&w.root.join(".opencode/commands/fix.md"), "---\ndescription: Fix it\n---\n");
    write(&w.root.join("opencode.jsonc"), "{\n  // a comment\n  \"command\": { \"deploy\": { \"template\": \"Deploy\", \"description\": \"Ship to prod\" }, },\n}\n");
    skill(&w.root.join(".claude/skills"), "proj-skill", "From the project");
    let l = offered("opencode", &w.root);
    assert_eq!(typed(&l), vec!["/customize-opencode", "/deploy", "/fix", "/my-skill", "/proj-skill"]);
    assert_eq!((about(&l, "/deploy"), about(&l, "/my-skill")), ("Ship to prod", "Mine"));
    assert_eq!(triggers("opencode"), "/");
}

#[test]
fn devin_names_a_skill_by_its_folder_tells_apart_two_places_with_one_name_and_reads_its_plugins() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let home = w.base.join("home");
    write(&home.join(".config/devin/skills/alpha/SKILL.md"), "---\nname: Alpha Shown\ndescription: Devin's alpha\n---\n");
    skill(&w.root.join(".devin/skills"), "beta", "The project's beta");
    skill(&w.root.join(".claude/skills"), "alpha", "Claude's alpha");
    write(&w.root.join(".devin/skills/model-only/SKILL.md"), "---\nname: model-only\ndescription: For Devin\ntriggers: [model]\n---\n");
    write(&home.join(".local/share/devin/cli/plugins/kit/plugin.json"), &json!({ "name": "kit" }).to_string());
    skill(&home.join(".local/share/devin/cli/plugins/kit/skills"), "tool", "A kit's tool");
    let l = offered("devin", &w.root);
    assert_eq!(typed(&l), vec!["/beta", "/claude:alpha", "/devin:alpha", "/kit:tool"]);
    // Claude Code's turned off in its config: the name is one place's again
    write(&w.root.join(".devin/config.json"), &json!({ "read_config_from": { "claude": false } }).to_string());
    assert_eq!(typed(&offered("devin", &w.root)), vec!["/alpha", "/beta", "/kit:tool"]);
}

#[test]
fn grok_needs_a_name_and_leaves_out_what_git_ignores_and_its_config_ignores() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let home = w.base.join("home");
    skill(&home.join(".grok/skills"), "g-user", "Yours");
    skill(&home.join(".claude/skills"), "g-claude", "Claude Code's");
    skill(&w.root.join(".grok/skills"), "g-project", "The project's");
    skill(&w.root.join(".grok/skills"), "secret", "Ignored by git");
    write(&w.root.join(".gitignore"), ".grok/skills/secret/\n");
    write(&w.root.join(".grok/skills/nameless/SKILL.md"), "---\ndescription: No name\n---\n");
    skill(&home.join("extra"), "g-extra", "From a path of its config");
    skill(&home.join("extra"), "g-skip", "Ignored by its config");
    write(&home.join(".grok/config.toml"), "[skills]\npaths = [\"~/extra\"]\nignore = [\"~/extra/g-skip\"]\n");
    assert_eq!(typed(&offered("grok", &w.root)), vec!["/g-claude", "/g-extra", "/g-project", "/g-user"]);
}

#[test]
fn hermes_spells_names_its_way_and_adds_trusted_projects_bundles_and_quick_commands() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let hermes = w.base.join("home/.hermes");
    write(&hermes.join("skills/web/Web_Tool/SKILL.md"), "---\nname: Web Tool\ndescription: Browse the web\n---\n");
    write(&hermes.join("skills/web/Web_Tool/references/inner/SKILL.md"), "---\nname: inner\ndescription: In a support folder\n---\n");
    skill(&hermes.join("skills"), "help", "Named like one of its commands");
    write(&hermes.join("skills/plain/SKILL.md"), "---\nname: plain\n---\n# Title\nDoes plain things\n");
    skill(&w.root.join(".hermes/skills"), "proj", "The project's");
    write(&hermes.join("skill-bundles/morning.yaml"), "name: Morning Set\nskills: [plain, proj]\n");
    write(&hermes.join("config.yaml"), "quick_commands:\n  deploy:\n    description: Deploy now\n");
    assert_eq!(typed(&offered("hermes", &w.root)), vec!["/deploy", "/morning-set", "/plain", "/web-tool"], "an untrusted project's are left out");
    write(&hermes.join("config.yaml"), &format!("skills:\n  trusted_project_dirs:\n    - {}\n  disabled: [plain]\n", w.root.display()));
    let l = offered("hermes", &w.root);
    assert_eq!(typed(&l), vec!["/morning-set", "/proj", "/web-tool"]);
    assert_eq!(about(&l, "/morning-set"), "Load 2 skills as a bundle");
}

#[test]
fn omp_offers_skills_as_skill_name_from_every_place_it_reads_its_commands_and_its_prompts() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let agent = w.base.join("pi"); // PI_CODING_AGENT_DIR, as omp reads it too
    skill(&agent.join("skills"), "o-agent", "Its own");
    skill(&w.root.join(".omp/skills"), "o-project", "The project's");
    skill(&w.base.join("claude/skills"), "o-claude", "Claude Code's");
    skill(&w.base.join("home/.agents/skills"), "o-agents", "In .agents");
    write(&agent.join("commands/run.md"), "---\ndescription: Run it\n---\n");
    write(&agent.join("commands/review.md"), "Named like one of its commands\n");
    write(&agent.join("prompts/sub/outline.md"), "---\ndescription: Plan the work\n---\n");
    let l = offered("omp", &w.root);
    assert_eq!(typed(&l), vec!["/outline", "/run", "/skill:o-agent", "/skill:o-agents", "/skill:o-claude", "/skill:o-project"]);
    assert_eq!(about(&l, "/outline"), "Plan the work (user:sub)");
    // its config turns Claude Code's off
    write(&agent.join("config.yml"), "skills:\n  enableClaudeUser: false\n");
    assert!(!typed(&offered("omp", &w.root)).contains(&"/skill:o-claude"));
}

#[test]
fn agy_offers_the_projects_yours_and_the_ones_it_comes_with_less_its_own_commands() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let gemini = w.base.join("home/.gemini");
    skill(&w.root.join(".agents/skills"), "a-project", "The project's");
    skill(&gemini.join("config/skills"), "a-user", "Yours");
    skill(&gemini.join("config/plugins/p/skills"), "a-plugin", "A plugin's");
    skill(&gemini.join("antigravity-cli/builtin/skills"), "help", "Its own command");
    skill(&gemini.join("antigravity-cli/builtin/skills"), "a-builtin", "It comes with it");
    write(&gemini.join("config/skills/quiet/SKILL.md"), "---\nname: quiet\ndescription: No slash\ndisable-slash-command: true\n---\n");
    skill(&w.base.join("home/named"), "a-named", "Named in skills.json");
    write(&gemini.join("config/skills.json"), &json!({ "entries": [{ "path": "~/named" }] }).to_string());
    assert_eq!(typed(&offered("agy", &w.root)), vec!["/a-builtin", "/a-named", "/a-plugin", "/a-project", "/a-user"]);
}
