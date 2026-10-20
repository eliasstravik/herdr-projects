//! Project-specific Mission Control spaces and the research approval gate.
use crate::common::*;
use serde_json::json;
use sw::herdr::Herdr;
use sw::json::*;
use sw::mission_control::apply_priority;
use sw::store::tasks::patch;

fn mission(w: &World) {
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", w.base.join("mission-coordinator")); }
    let api = Herdr::new(Some(w.socket.clone()));
    let ws = api.workspace_create(&w.root_str(), "proj", &[]).unwrap();
    sw::mission_layout::register_project(&w.socket, &w.root_str(), &ws.at("workspace").t("workspace_id"), &ws.at("root_pane").t("pane_id")).unwrap();
    sw::mission_layout::selected(&w.socket, &w.root_str()).unwrap();
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }

#[test]
fn opening_mission_control_creates_a_task_space_for_every_project_folder() {
    let w = world();
    let root = w.base.join("Developer");
    std::fs::create_dir_all(root.join("alpha")).unwrap();
    std::fs::create_dir_all(root.join("beta")).unwrap();
    unsafe { std::env::set_var("HERDR_CONTROL_PROJECTS_DIR", &root); }
    sw::mission_control::show(false).unwrap();
    let spaces = Herdr::new(Some(w.socket.clone())).workspaces().unwrap();
    assert_eq!(spaces.len(), 2);
    assert!(spaces.iter().any(|s| s.s("label") == "alpha"));
    assert!(spaces.iter().any(|s| s.s("label") == "beta"));
    assert_eq!(w.state().at("panes").as_array().unwrap().len(), 2);
    assert_eq!(sw::mission_layout::state(&w.socket).at("projects").as_object().map(|o| o.len()), Some(2));
}

#[test]
fn selecting_another_project_keeps_each_orchestrator_in_its_own_space() {
    let w = world(); apply_priority("agy,codex").unwrap();
    mission(&w); sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let first = sw::mission_control::project_coordinator(&w.root, &w.socket).unwrap();
    let first_state = sw::coordinator::state::get(first.clone()).unwrap();
    let api = Herdr::new(Some(w.socket.clone()));
    let other = w.base.join("other-project");
    std::fs::create_dir_all(&other).unwrap();
    git(&other, &["init", "-q", "-b", "main"]);
    std::fs::write(other.join("b.txt"), "one\n").unwrap();
    git(&other, &["add", "-A"]); git(&other, &["commit", "-qm", "init"]);
    let ws = api.workspace_create(other.to_str().unwrap(), "other-project", &[]).unwrap();
    sw::mission_layout::register_project(&w.socket, other.to_str().unwrap(), &ws.at("workspace").t("workspace_id"), &ws.at("root_pane").t("pane_id")).unwrap();
    sw::mission_layout::selected(&w.socket, other.to_str().unwrap()).unwrap();
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let second = sw::mission_control::project_coordinator(&other, &w.socket).unwrap();
    let second_state = sw::coordinator::state::get(second).unwrap();
    assert_ne!(first_state.s("pane"), second_state.s("pane"));
    assert_ne!(first_state.s("space"), second_state.s("space"));
    assert_eq!(api.workspace_get(first_state.s("space")).unwrap().s("label"), "proj");
    assert_eq!(api.workspace_get(second_state.s("space")).unwrap().s("label"), "other-project");
    assert!(api.pane_get(first_state.s("pane")).is_some(), "switching projects must preserve the previous coordinator pane");
    sw::mission_layout::selected(&w.socket, &w.root_str()).unwrap();
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    assert_eq!(sw::coordinator::state::get(first).unwrap().s("pane"), first_state.s("pane"));
}

#[test]
fn reviewed_research_report_returns_a_conclusion_and_waits_for_go_before_implementation() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let id = w.add("Orchestrator-Konzept untersuchen");
    let report_dir = sw::research::report_folder(&task(&id));
    std::fs::create_dir_all(&report_dir).unwrap();
    std::fs::write(report_dir.join("report.md"), "Fazit: drei konkrete Änderungen.\n").unwrap();
    patch(&id, |t| {
        t.put("research", true); t.put("stage", "done");
        t.put("report", json!({"file":"report.md","link":"https://reports.example/private"}));
        t.obj_mut("link").put("socket", w.socket.clone());
    }).unwrap();
    let completed = task(&id);
    sw::finish::research::mission_result(&completed, None); drain();
    let co = sw::mission_control::project_coordinator(&w.root, &w.socket).unwrap();
    let pane = sw::coordinator::state::get(co).unwrap().t("pane");
    let research_message = prompts(&w, &pane).last().unwrap().clone();
    assert!(research_message.contains("[MISSION-CONTROL-RESEARCH-RESULT]"));
    assert!(research_message.to_lowercase().contains("lies den vollständigen bericht"));
    assert!(research_message.contains("nichts umgesetzt"));
    assert!(research_message.contains("Go für Umsetzung"));
    assert_eq!(task(&id).s("missionImplementationApproval"), "pending");
    sw::mission_control::submit(&w.root, "Go für Umsetzung", &w.socket).unwrap(); drain();
    let implementation = prompts(&w, &pane).last().unwrap().clone();
    assert!(implementation.contains("[MISSION-CONTROL-UMSETZUNG-FREIGEGEBEN]"));
    assert!(implementation.contains("Freigegebener Recherche-Task"));
    assert!(implementation.contains("report.md"));
    assert_eq!(task(&id).s("missionImplementationApproval"), "approved");
    assert!(sw::mission_control::submit(&w.root, "Go für Umsetzung", &w.socket).is_err());
}
