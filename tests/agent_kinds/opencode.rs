//! OpenCode's full-screen launch keeps V1 flags and gives V2 a session selected before its first prompt.
use crate::common;
use common::*;
use serde_json::json;
use sw::agents::adapter::one_shot;
use sw::agents::start::{prepare, start_args, StartOpts};

fn version(v: &str) {
    let file = sw::store::files::data_dir().join("versions.json");
    let mut all = sw::store::files::read_json(&file).unwrap();
    all["opencode"] = json!({ "v": v, "at": sw::clock::now(), "found": true });
    sw::store::files::write_json(&file, &all).unwrap();
}

#[test]
fn opencode_v1_keeps_its_model_flag_and_resumed_conversation() {
    let w = world();
    version("1.18.34");
    for resume in ["", "ses_existing"] {
        let o = prepare("opencode", &w.root, StartOpts { model: "openai/gpt-6-astra".into(), resume: resume.into(), ..Default::default() }).unwrap();
        let args = start_args("opencode", &o).unwrap();
        let expected: Vec<&str> = if resume.is_empty() { vec!["--auto", "-m", "openai/gpt-6-astra"] } else { vec!["--session", resume, "--auto", "-m", "openai/gpt-6-astra"] };
        assert_eq!(args, expected);
    }
}

#[test]
fn opencode_headless_model_selection_still_uses_run() {
    let _w = world();
    let shot = one_shot("opencode", "openai/gpt-6-astra", "", "Summarize.", "", false).unwrap();
    assert_eq!(shot.command, "opencode");
    assert_eq!(shot.args, ["run", "-m", "openai/gpt-6-astra", "Summarize."]);
}
