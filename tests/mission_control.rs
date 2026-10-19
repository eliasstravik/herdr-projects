//! Mission Control uses the chosen CLI, keeps all agents beside its project input, and returns reviews to that session.
use crate::common;
use common::*;
use serde_json::json;
use sw::herdr::Herdr;
use sw::json::*;
use sw::mission_control::{apply_priority, parse_priority, submit};
use sw::store::settings::{change::set_value, lookup::{setting, setting_text, Scope}};

fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn mission(w: &World) -> String {
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", w.base.join("mission-coordinator")); }
    let api = Herdr::new(Some(w.socket.clone()));
    let ws = api.workspace_create(&w.root_str(), "Mission Control", &[]).unwrap();
    let pane = ws.at("root_pane").t("pane_id");
    sw::mission_layout::register(&w.socket, &pane).unwrap();
    pane
}
#[test]
fn priority_selects_the_coordinator_and_workers_and_refuses_invalid_changes_before_saving() {
    let _w = world();
    assert_eq!(parse_priority("antigravity-cli, qoder, codex").unwrap(), ["agy", "qodercli", "codex"]);
    apply_priority("agy,codex,freebuff").unwrap();
    assert_eq!(setting_text("coordinator.agent", None), "agy");
    assert_eq!(setting_text("worker.agent", None), "codex");
    assert_eq!(setting("worker.fallback", None), json!(["freebuff"]));
    assert_eq!(setting_text("review.by", None), "coordinator");
    assert_eq!(setting_text("review.when", None), "always");
    let before = std::fs::read(sw::mission_control::priority_file()).unwrap();
    assert!(apply_priority("agy,nonexistent").is_err());
    assert!(apply_priority("agy,agy/another-model").is_err());
    assert_eq!(std::fs::read(sw::mission_control::priority_file()).unwrap(), before);
    assert_eq!(setting_text("coordinator.agent", None), "agy");
}
#[test]
fn folders_are_visible_before_they_have_tasks_and_keep_their_real_origin() {
    let w = world();
    let root = w.base.join("Developer");
    std::fs::create_dir_all(root.join("Ordner ohne Git")).unwrap();
    std::fs::create_dir_all(root.join(".versteckt")).unwrap();
    std::fs::write(root.join("AGENTS.md"), "rules").unwrap();
    let projects = sw::mission_projects::scan(&root).unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "Ordner ohne Git");
    assert!(projects[0].remote.is_empty());
    git(&w.root, &["remote", "add", "origin", "https://github.com/example/proj.git"]);
    let real = sw::mission_projects::scan(&w.base).unwrap().into_iter().find(|p| p.path == w.root).unwrap();
    assert!(!real.remote.is_empty());
}
#[test]
fn opening_and_submitting_uses_the_selected_cli_and_project_in_one_visible_tab() {
    let w = world();
    apply_priority("agy,codex").unwrap();
    let root = mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let c = sw::coordinator::state::get(&w.socket).unwrap();
    assert_eq!(c.at("run").s("kind"), "agy");
    assert_ne!(c.s("pane"), root);
    assert!(c.b("tiled"));
    sw::mission_control::show(false).unwrap();
    assert_eq!(w.state().arr("workspaces").len(), 1, "opening twice focuses the existing Mission Control");
    submit(&w.root, "Baue eine Suche", &w.socket).unwrap(); drain();
    let c = sw::coordinator::state::get(&w.socket).unwrap();
    assert!(prompts(&w, c.s("pane")).last().unwrap().contains("Baue eine Suche"));
    let context = c.arr("where").last().unwrap();
    assert_eq!(context.s("root"), w.root_str());
    assert!(context.b("delegate"));
    let prompt = prompts(&w, c.s("pane")).last().unwrap().clone();
    assert!(prompt.contains("[MISSION-CONTROL-PROJEKT]"));
    assert!(prompt.contains(&w.root_str()));
    assert!(prompt.contains("Implementiere keine Änderungen selbst"));
    assert_eq!(w.state().arr("tabs").len(), 1);
    assert!(!calls(&w).iter().any(|c| c.starts_with("tab create")));
}
#[test]
fn worker_results_are_reviewed_by_the_existing_orchestrator_without_a_reviewer_cli() {
    let w = world();
    apply_priority("agy,codex").unwrap();
    let root = mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(&w.socket).unwrap().t("pane");
    let id = w.add("Ein Ergebnis prüfen");
    w.script(json!({"worker":[{"files":{"f.txt":"Änderung\n"},"note":"Fertig."}],
        "coordinator":[{"reviewTask":id,"verdict":{"meets":true,"text":"Änderung im selben Orchestrator geprüft.","findings":[]}}]}));
    delegate(&id);
    assert_eq!(task(&id).at("link").s("missionPane"), root);
    let t = settle(&id);
    assert_eq!(t.s("agent"), "codex");
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(sw::coordinator::state::get(&w.socket).unwrap().s("pane"), pane);
    assert!(t.at("review").s("by").contains("Orchestrator"));
    assert!(prompts(&w, &pane).last().unwrap().contains("[ORCHESTRATOR-REVIEW]"));
    assert_eq!(w.state().arr("tabs").len(), 1);
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("agent start")).count(), 2);
    assert!(!calls(&w).iter().any(|c| c.starts_with("tab create")));
}
#[test]
fn changing_priority_replaces_an_idle_coordinator_without_closing_mission_control() {
    let w = world(); apply_priority("agy,codex").unwrap();
    let root = mission(&w); sw::mission_control::ensure(&w.socket).unwrap(); drain();
    apply_priority("codex,agy").unwrap(); sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let c = sw::coordinator::state::get(&w.socket).unwrap();
    assert_eq!(c.at("run").s("kind"), "codex");
    assert!(Herdr::new(Some(w.socket.clone())).pane_get(&root).is_some());
    assert!(!calls(&w).iter().any(|c| c.starts_with("tab close")));
    set_value("review.when", &json!("never"), &Scope::you()).unwrap();
}

#[test]
fn coordinator_quota_handover_keeps_the_error_terminal_visible_and_follows_the_order() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let old = sw::coordinator::state::get(&w.socket).unwrap().t("pane");
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == old { p["agent"]["screen"] = json!("⏺ API Error: usage limit reached. Quota exceeded."); p["agent"]["status"] = json!("done"); } } });
    sw::mission_fallback::tick(&w.socket).unwrap(); drain();
    let c = sw::coordinator::state::get(&w.socket).unwrap();
    assert_eq!(c.at("run").s("kind"), "codex");
    assert_ne!(c.s("pane"), old);
    assert!(Herdr::new(Some(w.socket.clone())).pane_get(&old).is_some());
    assert_eq!(sw::mission_fallback::target(&w.socket).unwrap().s("kind"), "codex");
    assert!(prompts(&w, c.s("pane")).last().unwrap().contains("Providerwechsel"));
    assert_eq!(w.state().arr("tabs").len(), 1);
    assert!(!calls(&w).iter().any(|c| c.starts_with("tab close")));
}

#[test]
fn antigravity_trust_preserves_model_and_existing_workspaces_without_duplicate_entries() {
    let w = world();
    let file = sw::store::files::user_home().join(".gemini/antigravity-cli/settings.json");
    sw::store::files::write_json(&file, &json!({"model":"keep-my-model","trustedWorkspaces":["/already/trusted"],"other":true})).unwrap();
    assert!(sw::agents::adapter::trust("agy", &w.root));
    assert!(sw::agents::adapter::trust("agy", &w.root));
    let after = sw::store::files::read_json(&file).unwrap();
    assert_eq!(after.s("model"), "keep-my-model");
    assert!(after.b("other"));
    assert_eq!(after.arr("trustedWorkspaces").len(), 2);
    std::fs::write(&file, "{not json").unwrap();
    assert!(!sw::agents::adapter::trust("agy", &w.root));
    assert_eq!(read(&file), "{not json");
}
#[test]
fn a_review_request_waits_for_the_current_orchestrator_turn_to_finish() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(&w.socket).unwrap().t("pane");
    set_status(&w, &pane, "working");
    sw::coordinator::requests::send(&w.root, "[ORCHESTRATOR-REVIEW]\nPrüfe das Ergebnis", false, "", &w.socket).unwrap(); drain();
    assert!(prompts(&w, &pane).is_empty());
    assert_eq!(sw::coordinator::state::get(&w.socket).unwrap().arr("queue").len(), 1);
    set_status(&w, &pane, "idle");
    sw::coordinator::pump::pump(w.socket.as_str());
    assert!(prompts(&w, &pane).last().unwrap().contains("[ORCHESTRATOR-REVIEW]"));
}

#[test]
fn worker_choice_excludes_the_running_coordinator_even_after_its_provider_changes() {
    let w = world(); apply_priority("agy,codex,claude").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let id = w.add("Andere CLI");
    assert_eq!(sw::mission_control::worker_choice(&task(&id), &w.socket).unwrap(), "codex");
    sw::coordinator::state::change(&w.socket, |c| { c.obj_mut("run").put("kind", "codex"); true }).unwrap();
    assert!(!sw::mission_control::worker_allowed(&w.socket, "codex/another-model"));
    assert_eq!(sw::mission_control::worker_choice(&task(&id), &w.socket).unwrap(), "claude");
}

#[test]
fn a_project_message_waits_for_the_previous_turn_and_refuses_tasks_in_another_repo() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(&w.socket).unwrap().t("pane");
    set_status(&w, &pane, "working");
    submit(&w.root, "Ist die Startseite umgestellt?", &w.socket).unwrap(); drain();
    assert!(prompts(&w, &pane).is_empty());
    set_status(&w, &pane, "idle"); sw::coordinator::pump::pump(w.socket.as_str());
    assert_eq!(prompts(&w, &pane).len(), 1);
    assert_eq!(sw::mission_control::task_folder(&w.base, "", &pane, &w.socket).unwrap(), w.root);
    let other = w.base.join("other-project"); std::fs::create_dir(&other).unwrap();
    assert!(sw::mission_control::task_folder(&w.base, other.to_str().unwrap(), &pane, &w.socket).is_err());
    assert_eq!(sw::mission_control::task_folder(&w.base, other.to_str().unwrap(), "worker-pane", &w.socket).unwrap(), other);
}

#[test]
fn mission_control_only_draws_projects_and_input_with_tasks_in_the_native_sidebar() {
    use sw::mission_ui::{State, draw};
    let w = world();
    let (tx, _) = std::sync::mpsc::channel();
    let mut s = State {
        projects: vec![sw::mission_projects::Project {name: "proj".into(), path: w.root.clone(), remote: String::new()}],
        project: Default::default(), draft: Default::default(), order: Default::default(), priority: false, sidebar: false, sending: false,
        notice: String::new(), error: String::new(), coordinator: json!({"agent":"agy"}), drafts: Default::default(),
        folders_rect: Default::default(), input_rect: Default::default(), send_rect: Default::default(), priority_rect: Default::default(), refresh_rect: Default::default(),
        socket: w.socket.clone(), tx, closed_at: None,
    };
    s.project.select(Some(0));
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(104, 60)).unwrap();
    terminal.draw(|f| draw::draw(f, &mut s)).unwrap();
    let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("offene Tasks ganz links unter agents"));
    assert!(text.contains("Task für proj"));
    assert!(!text.contains("Orchestrator · Live-Ausgabe"));
    assert!(!text.contains("Noch keine Tasks"));
    assert!(s.folders_rect.width > 90);
}

#[test]
fn antigravity_individual_quota_marker_starts_the_next_coordinator_and_keeps_project_context() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    submit(&w.root, "Ist die Startseite umgestellt?", &w.socket).unwrap(); drain();
    let old = sw::coordinator::state::get(&w.socket).unwrap().t("pane");
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == old { p["agent"]["screen"] = json!("⚠ Individual quota reached. Please upgrade your subscription. Resets in 4h."); p["agent"]["status"] = json!("done"); } } });
    sw::mission_fallback::tick(&w.socket).unwrap(); drain();
    let c = sw::coordinator::state::get(&w.socket).unwrap();
    assert_eq!(c.at("run").s("kind"), "codex");
    let messages = prompts(&w, c.s("pane"));
    assert!(messages.last().unwrap().contains(&w.root_str()));
    assert!(messages.last().unwrap().contains("Ist die Startseite umgestellt?"));
    assert!(Herdr::new(Some(w.socket.clone())).pane_get(&old).is_some());
}

#[test]
fn resting_old_coordinators_are_saved_and_closed_after_handover_while_workers_stay() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let api = Herdr::new(Some(w.socket.clone()));
    let old = sw::coordinator::state::get(&w.socket).unwrap();
    let record = sw::coordinator::state::dir_of(&w.socket).join("handover-test.json");
    sw::store::files::write_json(&record, &json!(old)).unwrap();
    let worker = api.pane_split(old.s("pane"), &w.root_str(), "down", &[]).unwrap();
    let next = api.pane_split(old.s("pane"), old.s("cwd"), "down", &[]).unwrap();
    sw::coordinator::state::change(&w.socket, |c| {c.put("pane", next.at("pane_id").clone());true}).unwrap();
    api.call(sw::herdr::cli::args(&["agent","start","next","--kind","codex","--pane",next.s("pane_id")])).unwrap();
    set_status(&w, old.s("pane"), "working");
    sw::mission_panes::tidy(&w.socket).unwrap();
    assert!(api.pane_get(old.s("pane")).is_some());
    set_status(&w, old.s("pane"), "idle");
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == old.s("pane") { p["terminal_id"] = json!("reused-terminal"); } } });
    sw::mission_panes::tidy(&w.socket).unwrap();
    assert!(api.pane_get(old.s("pane")).is_some(), "never close a reused pane ID");
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == old.s("pane") { p["terminal_id"] = old.at("terminal").clone(); } } });
    sw::mission_panes::tidy(&w.socket).unwrap();
    assert!(api.pane_get(old.s("pane")).is_none());
    assert!(record.with_extension("terminal.txt").exists());
    assert!(api.pane_get(worker.s("pane_id")).is_some());
    assert!(api.pane_get(next.s("pane_id")).is_some());
}

#[test]
fn stopped_tasks_stay_in_the_native_agent_list_without_becoming_running_coding_agents() {
    let w = world(); apply_priority("agy,codex").unwrap(); let root = mission(&w);
    let api = Herdr::new(Some(w.socket.clone()));
    let pane = api.pane_split(&root, &w.root_str(), "right", &[]).unwrap();
    let id = w.add("Suchseite");
    sw::store::tasks::set(&id, json!({"stage":"failed","link":{"socket":w.socket,"pane":pane.s("pane_id"),"terminal":pane.s("terminal_id"),"missionPane":root}})).unwrap();
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane.s("pane_id") { p["busy"] = json!(true); } } });
    sw::mission_agents::sync(&[task(&id)], &w.socket);
    let raw = api.call(sw::herdr::cli::args(&["agent","get",pane.s("pane_id")])).unwrap();
    assert_eq!(raw.at("agent").s("agent"), sw::herdr::api::TASK_AGENT);
    assert!(api.agent_get(pane.s("pane_id")).unwrap().is_none());
    assert!(api.agents().unwrap().is_empty());
    assert!(w.state().arr("metadata").iter().any(|m| m.s("pane") == pane.s("pane_id") && m.to_string().contains("task=PRJ-1 Suchseite")));
    sw::mission_agents::sync(&[], &w.socket);
    assert!(api.call(sw::herdr::cli::args(&["agent","get",pane.s("pane_id")])).is_err());
}
