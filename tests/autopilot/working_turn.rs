//! A wake into a coordinator that finished a turn before: herdr leaves `completion_seq` out while its turn is going,
//! and the turn is still the wake's, so what it starts in that turn is counted as autopilot's; a turn typed straight
//! into its terminal after the wake's ended is not.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::json::*;

#[test]
fn a_task_it_starts_while_herdr_leaves_completion_seq_out_is_counted_as_started() {
    let w = pilot_world();
    let co = proj(&w);
    // a turn of its own, finished: herdr now says how many it finished
    w.script(json!({ "coordinator": [{ "said": "Hello." }, { "status": "working" }] }));
    ok(&["coordinator", "tell", "--project", "proj", "hello"]);
    drain();
    let yours = w.add("Yours");
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.scope", "all", "--autopilot.every", "none"]);
    tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2);
    let a = sw::herdr::Herdr::new(Some(w.socket.clone())).agent_get(&pane_of(&co)).unwrap().unwrap();
    assert_eq!(a.s("agent_status"), "working"); assert!(a.at("completion_seq").is_null(), "{a}");
    in_pane(&w, &pane_of(&co));
    assert!(sw::autopilot::turn::autopilot_turn(&co), "the wake's turn is still going");
    w.script(json!({ "worker": [step(1)] }));
    ok(&["delegate", &yours]);
    in_pane(&w, "");
    assert_eq!(sw::autopilot::record::task_rec(&record(&co), &yours).map(|r| r.t("by")).unwrap_or_default(), "started");
    // its turn ends, then you type straight into its terminal: herdr again leaves completion_seq out, and the turn is yours
    end_turn(&w, &co);
    tick(&w);
    busy(&w, &co);
    assert!(!sw::autopilot::turn::autopilot_turn(&co), "a turn after the wake's is not autopilot's");
}
