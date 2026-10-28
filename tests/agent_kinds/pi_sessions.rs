//! pi's saved conversations are looked for only in the folder pi keeps for the agent's working folder, never in every
//! folder of a long history (#393): a task's transcript and the newest conversation of a folder are both found there.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::agents::adapter::{latest_session, transcript};

#[test]
fn a_pi_conversation_is_found_in_its_own_folders_place_and_not_in_another_folders() {
    let _w = world_with(Opts::default());
    let sessions = Path::new(&std::env::var("PI_CODING_AGENT_DIR").unwrap()).join("sessions");
    let (mine, other) = (Path::new("/work/app"), Path::new("/work/other"));
    let save = |cwd: &Path, id: &str| { let f = sessions.join(pi_folder(cwd)).join(format!("2026-10-28T00-00-00-000Z_{id}.jsonl")); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(&f, format!("{}\n", json!({ "type": "session", "id": id, "cwd": cwd }))).unwrap(); f };
    let f = save(mine, "sw-mine");
    save(other, "sw-theirs");
    assert_eq!(transcript("pi", "sw-mine", mine), Some(f));
    assert_eq!(transcript("pi", "sw-theirs", mine), None, "another folder's is not looked through");
    assert_eq!(latest_session("pi", mine, 0, None).as_deref(), Some("sw-mine"));
}
