//! Mission Control startup, one global CLI priority for coordinator and workers, and project-scoped task submission.
use crate::coordinator::{co::Co, requests, state, who};
use crate::err::Res;
use crate::herdr::{Herdr, cli::current_socket};
use crate::json::*;
use crate::store::files::{board_env, user_home, write_atomic};
use crate::store::settings::{change::set_many, lookup::Scope};
use crate::util::shq;
use std::path::{Path, PathBuf};

pub const POOL: &[&str] = &["freebuff", "codebuddy", "claude", "codex", "agy", "cursor", "opencode", "kiro", "cline", "qodercli", "grok"];
pub fn priority_file() -> PathBuf {
    std::env::var("HERDR_CONTROL_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| user_home().join(".config")).join("herdr/plugins/config/herdr-control")
    }).join("priority.txt")
}
pub fn priority() -> String { std::fs::read_to_string(priority_file()).unwrap_or_else(|_| POOL.join(",")).trim().to_string() }
pub fn parse_priority(text: &str) -> Res<Vec<String>> {
    let mut order = vec![];
    for value in text.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (name, suffix) = value.split_once('/').map(|(n, m)| (n, format!("/{m}"))).unwrap_or((value, String::new()));
        let name = match name { "antigravity" | "antigravity-cli" => "agy", "qoder" => "qodercli", "gemini" => crate::fail!("Bitte agy für Antigravity CLI verwenden."), n => n };
        if !POOL.contains(&name) || value.chars().any(char::is_whitespace) { crate::fail!("Unbekannter CLI-Eintrag: {value}. Erlaubt: {}", POOL.join(", ")) }
        let on = format!("{name}{suffix}");
        if order.contains(&on) { crate::fail!("Doppelter Eintrag: {on}") }
        order.push(on);
    }
    if order.is_empty() { crate::fail!("Bitte mindestens eine CLI angeben.") }
    Ok(order)
}
/// All values are validated together before settings or the saved order change.
pub fn apply_priority(text: &str) -> Res<String> {
    let order = parse_priority(text)?;
    let primary = &order[0];
    let workers: Vec<_> = order.iter().filter(|a| crate::store::settings::values::agent_of(a).kind != crate::store::settings::values::agent_of(primary).kind).collect();
    let Some(worker) = workers.first() else { crate::fail!("Orchestrator und Worker brauchen mindestens zwei unterschiedliche CLIs.") };
    let fallback = if workers.len() > 1 { workers[1..].iter().map(|a| a.as_str()).collect::<Vec<_>>().join(",") } else { "none".into() };
    set_many(&[
        ("worker.agent".into(), json!(worker)), ("worker.fallback".into(), json!(fallback)),
        ("coordinator.agent".into(), json!(primary)), ("watcher.agent".into(), json!("none")),
        ("review.by".into(), json!("coordinator")), ("review.when".into(), json!("always")),
    ], &Scope::you())?;
    write_atomic(&priority_file(), format!("{}\n", order.join(",")).as_bytes())?;
    Ok(primary.clone())
}
/// Changing priority while the coordinator works is deferred until its terminal is at rest.
pub fn ensure(socket: &str) -> Res<String> {
    let Some(folder) = selected_folder(socket) else { return Ok("Projekt wählen".into()) };
    let co = project_coordinator(&folder, socket)?;
    crate::mission_fallback::tick_one(&co)?;
    let desired = crate::mission_fallback::target_for(&co).ok_or_else(|| crate::err::Error::new("Kein Orchestrator konfiguriert."))?;
    let api = Herdr::new(Some(socket.into()));
    let old = state::get(&co).unwrap_or_default();
    let live = api.agent_get(old.s("pane")).ok().flatten();
    if old.o("run").map(|r| !who::same_choice(r, &desired)).unwrap_or(false) {
        if live.as_ref().map(|a| !["idle", "done"].contains(&a.s("agent_status"))).unwrap_or(false) {
            return Ok(format!("{} arbeitet noch; danach Wechsel zu {}", old.at("run").s("kind"), desired.s("kind")));
        }
        if !state::pumping(&old) && !old.b("want") { requests::fresh(crate::mission_fallback::target_for(&co), &co)?; }
    } else if live.is_none() && !state::pumping(&old) && !old.b("want") && !old.b("error") { requests::open(&co)?; }
    crate::mission_panes::tidy(socket)?;
    Ok(format!("Orchestrator für {}: {}", folder.file_name().unwrap_or_default().to_string_lossy(), desired.s("kind")))
}
pub fn has_session(socket: &str) -> bool {
    selected_folder(socket).and_then(|folder| project_coordinator(&folder, socket).ok())
        .and_then(|co| state::get(&co)).is_some()
}
pub fn selected_folder(socket: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("HERDR_CONTROL_PROJECT_PATH") {
        let path = PathBuf::from(path);
        return path.is_dir().then(|| std::fs::canonicalize(&path).unwrap_or(path));
    }
    let layout = crate::mission_layout::state(socket);
    let path: PathBuf = if layout.s("selected").is_empty() {
        layout.at("projects").as_object()?.values().next()?.t("folder").into()
    } else { PathBuf::from(layout.s("selected")) };
    path.is_dir().then(|| std::fs::canonicalize(&path).unwrap_or(path))
}
pub fn is_active(socket: &str) -> bool { crate::mission_layout::state(socket).at("projects").as_object().map(|p| !p.is_empty()).unwrap_or(false) }
pub fn selected_coordinator(socket: &str) -> Option<Co> {
    selected_folder(socket).and_then(|folder| project_coordinator(&folder, socket).ok())
}
pub fn selected_summary(socket: &str) -> Value {
    selected_coordinator(socket).map(|co| crate::coordinator::look::summary(&co)).unwrap_or_else(|| crate::coordinator::look::summary(socket))
}
pub fn project_coordinator(folder: &Path, socket: &str) -> Res<Co> {
    let (repo, _) = crate::store::repos::repo_of(folder)?;
    Ok(Co::project(socket, &repo))
}
pub fn coordinator_for_task(t: &crate::store::tasks::Task, socket: &str) -> Option<Co> {
    let folder = crate::store::repos::folder_of(t.s("id"));
    project_coordinator(&folder, socket).ok()
}
pub fn pane_for_task(t: &crate::store::tasks::Task, socket: &str) -> String {
    coordinator_for_task(t, socket).and_then(|co| state::get(co).map(|c| c.t("pane"))).filter(|p| !p.is_empty()).unwrap_or_default()
}
/// Open a worker beside the persistent coordinator pane in that project's dedicated Herdr space.
pub fn split_task(api: &Herdr, folder: &Path, cwd: &str, env: &[(String, String)]) -> Res<Option<Value>> {
    let socket = api.socket_path();
    if !is_active(&socket) { return Ok(None); }
    let co = project_coordinator(folder, &socket)?;
    let Some(state) = state::get(&co) else { return Ok(None) };
    let root = state.t("pane");
    if root.is_empty() || api.pane_get(&root).is_none() { return Ok(None) }
    crate::store::files::with_lock_wait(&format!("mission-project-layout:{}", co.key()), 10000, || {
        let Some(control) = api.pane_get(&root) else { return Ok(None) };
        let layout = api.pane_layout(&root).unwrap_or(Value::Null);
        let children: Vec<Value> = state.arr("children").iter().filter(|id| api.pane_get(&text(id)).map(|p| p.s("tab_id") == control.s("tab_id")).unwrap_or(false)).cloned().collect();
        let largest = layout.arr("panes").iter().filter(|p| children.iter().any(|id| text(id) == p.s("pane_id")))
            .max_by_key(|p| p.at("rect").i("width") * p.at("rect").i("height"));
        let (target, direction) = match largest {
            Some(p) => (p.s("pane_id"), if p.at("rect").i("width") > p.at("rect").i("height") * 2 { "right" } else { "down" }),
            None => (root.as_str(), "right"),
        };
        let pane = api.pane_split(target, cwd, direction, env)?;
        if !pane.b("pane_id") { crate::fail!("Herdr hat kein Terminal-Pane für das Projekt geöffnet.") }
        state::change(&co, |c| {
            let mut children: Vec<Value> = c.arr("children").iter().filter(|id| api.pane_get(&text(id)).map(|p| p.s("tab_id") == control.s("tab_id")).unwrap_or(false)).cloned().collect();
            children.push(pane.at("pane_id").clone()); c.put("children", children); true
        })?;
        Ok(Some(pane))
    })
}
pub fn submit(folder: &Path, text: &str, socket: &str) -> Res<()> {
    if text.trim().is_empty() { crate::fail!("Bitte einen Task eingeben.") }
    if !folder.is_dir() { crate::fail!("Der Projektordner ist nicht mehr vorhanden: {}", folder.display()) }
    let co = project_coordinator(&folder, socket)?;
    let actual = state::get(&co).unwrap_or_default();
    if let (Some(run), Some(desired)) = (actual.o("run"), crate::mission_fallback::target_for(&co)) {
        if !who::same_choice(run, &desired) { crate::fail!("Der Wechsel zur gewählten CLI ist noch offen. Der Task bleibt im Eingabefeld.") }
    }
    let folder = std::fs::canonicalize(folder)?;
    let tasks: Vec<_> = crate::store::tasks::list(Default::default()).iter()
        .filter(|t| crate::store::repos::folder_of(t.s("id")) == folder && !t.b("archivedAt"))
        .map(|t| format!("{}: {} ({})", t.s("id"), t.s("title"), t.s("stage"))).collect();
    let approved_report = implementation_go(text).then(|| crate::store::tasks::list(Default::default()).into_iter()
        .filter(|t| crate::store::repos::folder_of(t.s("id")) == folder && crate::research::is_research(t)
            && t.s("stage") == "done" && t.s("missionImplementationApproval") == "pending" && t.at("report").is_object())
        .max_by(|a, b| a.f("created").partial_cmp(&b.f("created")).unwrap_or(std::cmp::Ordering::Equal)));
    if implementation_go(text) && approved_report.as_ref().and_then(|t| t.as_ref()).is_none() {
        crate::fail!("Für dieses Projekt gibt es keinen abgeschlossenen Recherchebericht, der auf dein Go wartet.")
    }
    let (message_tag, instructions, report_context) = if let Some(Some(t)) = &approved_report {
        let report = &t.at("report");
        ("[MISSION-CONTROL-UMSETZUNG-FREIGEGEBEN]", "Der Mensch gibt die Umsetzung des folgenden geprüften Rechercheberichts ausdrücklich frei. Lies den vollständigen Bericht, zerlege die dort empfohlene Umsetzung in konkrete Tasks und delegiere sie sofort an Worker mit anderer CLI. Übernimm keine Umsetzung selbst. Begrenze die Tasks auf die freigegebenen Empfehlungen; frage nach, wenn der Bericht mehrere deutlich unterschiedliche Umsetzungspfade offenlässt.", format!("Freigegebener Recherche-Task {} („{}“): {}", t.s("id"), t.s("title"), report.s("file")))
    } else {
        ("[MISSION-CONTROL-PROJEKT]", "Du koordinierst und reviewst ausschließlich. Implementiere keine Änderungen selbst. Delegiere jede Umsetzung mit `board add ... --repo {} --delegate` an einen Worker mit anderer CLI; der Workflow wählt diese aus der Prioritätenliste.", String::new())
    };
    let report_link = approved_report.as_ref().and_then(|t| t.as_ref()).map(|t| format!("Berichtdatei: {}. Berichtlink: {}.", crate::research::report_folder(t).join(t.at("report").s("file")).display(), t.at("report").s("link"))).unwrap_or_default();
    let prompt = format!("{message_tag}\nProjekt: {}\nOrdner: {}\nTasks dieses Projekts: {}\nDiese Nachricht gilt ausschließlich diesem Projekt. Beziehe Pronomen und Statusfragen nur darauf; Tasks anderer Projekte aus der bisherigen Unterhaltung sind hier kein Bezug. Prüfe bei fehlendem Task den Stand dieses Projekts.\n{instructions} {}\n{}\n{}\n\nNachricht des Menschen:\n{text}", folder.file_name().unwrap_or_default().to_string_lossy(), folder.display(), if tasks.is_empty() { "keine".into() } else { tasks.join("; ") }, shq(&folder.display().to_string()), report_context, report_link);
    requests::send(&folder, &prompt, true, "", &co)?;
    if let Some(Some(t)) = approved_report { crate::store::tasks::patch(t.s("id"), |task| task.put("missionImplementationApproval", "approved"))?; }
    Ok(())
}

/// Only a clear approval phrase can release a pending research report into implementation.
fn implementation_go(text: &str) -> bool {
    matches!(text.trim().to_lowercase().as_str(), "go" | "go!" | "go für umsetzung" | "umsetzung starten" | "umsetzung freigeben")
}
/// One persistent workspace, one root control pane; every CLI later splits that tab.
pub fn show(focus: bool) -> Res<()> {
    let socket = current_socket();
    crate::store::files::with_lock_wait(&format!("mission-open:{}", crate::herdr::cli::session_name(&socket)), 5000, || show_once(focus))
}
fn show_once(focus: bool) -> Res<()> {
    let socket = current_socket();
    let api = Herdr::new(Some(socket.clone()));
    apply_priority(&priority())?;
    let root = crate::mission_projects::root();
    if !root.is_dir() { crate::fail!("Projektordner fehlt: {}", root.display()) }
    let folders = crate::mission_projects::scan(&root)?;
    if folders.is_empty() { crate::fail!("Unter {} wurden keine Projektordner gefunden.", root.display()) }
    let workspaces: Vec<Value> = api.workspaces()?.into_iter().filter(|w| !w.at("worktree").is_object()).collect();
    let mut focus_pane = String::new();
    for project in folders {
        let path = std::fs::canonicalize(&project.path).unwrap_or(project.path.clone()).display().to_string();
        let label = format!("{}", project.name);
        let saved = crate::mission_layout::state(&socket).at("projects").get(&crate::mission_layout::project_key(&path)).cloned().unwrap_or_default();
        let mut pane = api.pane_get(saved.s("pane"));
        let workspace = if let Some(p) = pane.as_ref() { p.t("workspace_id") }
        else if saved.b("workspace") && workspaces.iter().any(|w| w.s("workspace_id") == saved.s("workspace")) { saved.t("workspace") }
        else { workspaces.iter().find(|w| w.s("label") == label).map(|w| w.t("workspace_id")).unwrap_or_default() };
        let (workspace, pane) = if workspace.is_empty() {
            let ws = api.workspace_create(&path, &label, &board_env())?;
            let id = ws.at("root_pane").t("pane_id");
            if id.is_empty() { crate::fail!("Herdr hat kein Task-Pane für {} geöffnet.", project.name) }
            (ws.at("workspace").t("workspace_id"), ws.at("root_pane").clone())
        } else {
            let p = pane.take().or_else(|| api.panes(Some(&workspace)).ok()?.into_iter().find(|p| p.s("cwd") == path));
            let p = match p { Some(p) => p, None => { let r = api.tab_create(&workspace, &path, "Mission Control", &board_env())?; r.at("root_pane").clone() } };
            (workspace, p)
        };
        let pane_id = pane.t("pane_id");
        crate::mission_layout::prepare_project(&socket, &path, &workspace, &pane_id)?;
        if api.pane_idle(&pane_id) {
            let cmd = format!("HERDR_CONTROL_PROJECT_PATH={} {} control", shq(&path), shq(&crate::store::files::bin().display().to_string()));
            api.pane_run(&pane_id, &cmd)?;
        }
        if focus_pane.is_empty() { focus_pane = pane_id; }
    }
    if focus && !focus_pane.is_empty() {
        if let Some(pane) = api.pane_get(&focus_pane) { api.workspace_focus(pane.s("workspace_id"))?; api.tab_focus(pane.s("tab_id"))?; api.pane_focus(&focus_pane)?; }
    }
    Ok(())
}
pub fn rules(co: &Co) -> String {
    let socket = &co.socket;
    if !is_active(socket) { return String::new() }
    "\n\nMISSION CONTROL: Du bist der dauerhaft sichtbare Orchestrator. Antworte auf Deutsch. Der Mensch wählt links einen lokalen Projektordner und schickt unten einen Task. `board coordinator where` enthält den ausgewählten Ordner für jede Nachricht. Implementiere niemals selbst, editiere keine Projektdateien und übernimm keine Worker-Aufgabe. Du koordinierst und reviewst ausschließlich. Jede MISSION-CONTROL-PROJEKT-Nachricht trägt ihren verbindlichen Projektordner; verwende niemals Tasks anderer Projekte als Bezug. Zerlege freigegebene Umsetzungsaufträge in unabhängige Teilaufgaben, lege sie mit `board add ... --repo <vollständiger Ordner> --delegate` an und nutze `--after` für Abhängigkeiten. Lies die AGENTS.md des Projekts und die übergeordnete Developer/AGENTS.md. Prüfe vor Code-Arbeit den Git-Remote; wenn er fehlt, kläre das GitHub-Ziel sichtbar vor dem Start. Ersetze keinen vorhandenen Remote. Verwende für jede Aufgabe die globale worker.agent/worker.fallback-Reihenfolge, wobei die laufende Orchestrator-CLI ausgeschlossen ist; wähle keine anderen Rollen-Modelle. Alle Arbeit läuft in den von Herdr geöffneten echten interaktiven Terminal-Panes. Starte keine unsichtbaren Subagenten, keine ACP-Verbindungen und keine headless/print/exec-Modi. Verstecke Fehler nicht: nenne die Task-ID und die Ursache; die Terminal-Panes bleiben sichtbar. Die Ergebnisse der Teilaufgaben werden dir automatisch in dieser Sitzung zur Prüfung vorgelegt. Wenn eine Nachricht mit [ORCHESTRATOR-REVIEW] beginnt, prüfe selbst den genannten Arbeitsstand und schreibe das verlangte Urteil an den angegebenen Pfad; delegiere dieses Review nicht. Der Workflow verarbeitet dein Urteil und schickt notwendige Korrekturen an den Worker zurück. Bei [MISSION-CONTROL-RESEARCH-RESULT] lies den ganzen Bericht, gib dem Menschen ein klares Fazit, die wichtigsten Befunde, einen kurzen Umsetzungsvorschlag und den Berichtlink. Sage ausdrücklich, dass noch nichts umgesetzt wurde, und frage nach „Go für Umsetzung“. Lege bis dahin keine Umsetzungstasks an. Bei [MISSION-CONTROL-UMSETZUNG-FREIGEGEBEN] hat der Mensch die Umsetzung des genannten Rechercheberichts freigegeben: lies ihn vollständig und delegiere die begrenzte Umsetzung sofort an Worker; behandle das nicht als neue Recherche. Berichte dem Menschen Fortschritt, Blockaden und Review-Ergebnisse. Änderungen werden nach erfolgreichen Prüfungen vom Workflow committed und auf den GitHub-Remote gepusht; ein Merge braucht weiterhin die Freigabe des Menschen.\n".into()
}

/// The running coordinator's CLI is reserved for coordination, including after a provider handover.
pub fn worker_allowed(socket: &str, agent: &str) -> bool {
    if !is_active(socket) { return true; }
    let coordinator = selected_coordinator(socket).and_then(state::get).unwrap_or_default();
    let kind = if coordinator.at("run").b("kind") { coordinator.at("run").t("kind") }
        else { crate::mission_fallback::target(socket).map(|r| r.t("kind")).unwrap_or_default() };
    crate::store::settings::values::agent_of(agent).kind != kind
}
pub fn worker_allowed_for(t: &crate::store::tasks::Task, socket: &str, agent: &str) -> bool {
    if !is_active(socket) { return true; }
    let co = coordinator_for_task(t, socket).unwrap_or_else(|| Co::top(socket));
    let coordinator = state::get(&co).unwrap_or_default();
    let kind = if coordinator.at("run").b("kind") { coordinator.at("run").t("kind") }
        else { crate::mission_fallback::target_for(&co).map(|r| r.t("kind")).unwrap_or_default() };
    crate::store::settings::values::agent_of(agent).kind != kind
}
pub fn worker_choice(t: &crate::store::tasks::Task, socket: &str) -> Res<String> {
    let w = crate::store::settings::lookup::work(Some(t));
    let mut order = vec![if t.b("on") { t.t("on") } else { w.t("worker.agent") }];
    order.extend(crate::store::settings::values::list_of(w.at("worker.fallback")));
    // The global list still supplies a candidate when the coordinator falls back onto the worker's first CLI.
    order.extend(parse_priority(&priority())?);
    order.into_iter().find(|a| worker_allowed_for(t, socket, a) && crate::finish::hand_over::usable(a, w.s("worker.access")).is_empty())
        .ok_or_else(|| crate::err::Error::new("Keine andere verfügbare Worker-CLI. Bitte die CLI-Reihenfolge ergänzen."))
}

/// Tasks added from the coordinator inherit the message's project; a conflicting explicit repo is refused.
pub fn task_folder(folder: &Path, repo: &str, pane: &str, socket: &str) -> Res<PathBuf> {
    let requested = crate::store::repos::folder_for(repo, folder)?;
    if pane.is_empty() || !is_active(socket) { return Ok(requested); }
    let Some(co) = state::all(socket).into_iter().find(|co| state::get(co).map(|c| c.s("pane") == pane).unwrap_or(false)) else { return Ok(requested) };
    let co = state::get(&co).unwrap_or_default();
    let Some(message) = co.arr("where").last().filter(|m| m.s("said").starts_with("[MISSION-CONTROL-PROJEKT]") || m.s("said").starts_with("[MISSION-CONTROL-UMSETZUNG-FREIGEGEBEN]")) else { return Ok(requested) };
    let root = PathBuf::from(message.s("root"));
    if !repo.is_empty() && crate::store::repos::primary_root(&requested) != crate::store::repos::primary_root(&root) {
        crate::fail!("Dieser Auftrag gehört zu {}. Ein Task in {} wurde nicht angelegt. Verwende --repo {}.", root.display(), requested.display(), shq(&root.display().to_string()));
    }
    Ok(root)
}
