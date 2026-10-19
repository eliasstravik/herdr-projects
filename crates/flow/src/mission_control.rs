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
    let co = Co::top(socket);
    crate::mission_fallback::tick(socket)?;
    let desired = crate::mission_fallback::target(socket).ok_or_else(|| crate::err::Error::new("Kein Orchestrator konfiguriert."))?;
    let api = Herdr::new(Some(socket.into()));
    let old = state::get(&co).unwrap_or_default();
    let live = api.agent_get(old.s("pane")).ok().flatten();
    if old.o("run").map(|r| !who::same_choice(r, &desired)).unwrap_or(false) {
        if live.as_ref().map(|a| !["idle", "done"].contains(&a.s("agent_status"))).unwrap_or(false) {
            return Ok(format!("{} arbeitet noch; danach Wechsel zu {}", old.at("run").s("kind"), desired.s("kind")));
        }
        if !state::pumping(&old) && !old.b("want") { requests::fresh(crate::mission_fallback::target(socket), &co)?; }
    } else if live.is_none() && !state::pumping(&old) && !old.b("want") && !old.b("error") { requests::open(&co)?; }
    crate::mission_panes::tidy(socket)?;
    Ok(format!("Orchestrator: {}", desired.s("kind")))
}
pub fn submit(folder: &Path, text: &str, socket: &str) -> Res<()> {
    if text.trim().is_empty() { crate::fail!("Bitte einen Task eingeben.") }
    if !folder.is_dir() { crate::fail!("Der Projektordner ist nicht mehr vorhanden: {}", folder.display()) }
    let co = Co::top(socket);
    let actual = state::get(&co).unwrap_or_default();
    if let (Some(run), Some(desired)) = (actual.o("run"), crate::mission_fallback::target(socket)) {
        if !who::same_choice(run, &desired) { crate::fail!("Der Wechsel zur gewählten CLI ist noch offen. Der Task bleibt im Eingabefeld.") }
    }
    let folder = std::fs::canonicalize(folder)?;
    let tasks: Vec<_> = crate::store::tasks::list(Default::default()).iter()
        .filter(|t| crate::store::repos::folder_of(t.s("id")) == folder && !t.b("archivedAt"))
        .map(|t| format!("{}: {} ({})", t.s("id"), t.s("title"), t.s("stage"))).collect();
    let prompt = format!("[MISSION-CONTROL-PROJEKT]\nProjekt: {}\nOrdner: {}\nTasks dieses Projekts: {}\nDiese Nachricht gilt ausschließlich diesem Projekt. Beziehe Pronomen und Statusfragen nur darauf; Tasks anderer Projekte aus der bisherigen Unterhaltung sind hier kein Bezug. Prüfe bei fehlendem Task den Stand dieses Projekts.\nDu koordinierst und reviewst ausschließlich. Implementiere keine Änderungen selbst. Delegiere jede Umsetzung mit `board add ... --repo {} --delegate` an einen Worker mit anderer CLI; der Workflow wählt diese aus der Prioritätenliste.\n\nNachricht des Menschen:\n{text}", folder.file_name().unwrap_or_default().to_string_lossy(), folder.display(), if tasks.is_empty() { "keine".into() } else { tasks.join("; ") }, shq(&folder.display().to_string()));
    requests::send(&folder, &prompt, true, "", &co)?;
    Ok(())
}
/// One persistent workspace, one root control pane; every CLI later splits that tab.
pub fn show(focus: bool) -> Res<()> {
    let socket = current_socket();
    crate::store::files::with_lock_wait(&format!("mission-open:{}", crate::herdr::cli::session_name(&socket)), 5000, || show_once(focus))
}
fn show_once(focus: bool) -> Res<()> {
    let socket = current_socket();
    let api = Herdr::new(Some(socket.clone()));
    if let Some(pane) = crate::mission_layout::anchor(&socket).and_then(|id| api.pane_get(&id)) {
        if focus { api.workspace_focus(pane.s("workspace_id"))?; api.tab_focus(pane.s("tab_id"))?; api.pane_focus(pane.s("pane_id"))?; }
        return Ok(());
    }
    let root = crate::mission_projects::root();
    if !root.is_dir() { crate::fail!("Projektordner fehlt: {}", root.display()) }
    let saved = crate::mission_layout::state(&socket);
    if let Some(pane) = api.pane_get(saved.s("pane")).filter(|p| api.pane_idle(p.s("pane_id"))) {
        crate::mission_layout::register(&socket, pane.s("pane_id"))?;
        api.pane_run(pane.s("pane_id"), &format!("{} control", shq(&crate::store::files::bin().display().to_string())))?;
        if focus { api.workspace_focus(pane.s("workspace_id"))?; api.tab_focus(pane.s("tab_id"))?; api.pane_focus(pane.s("pane_id"))?; }
        return Ok(());
    }
    let ws = api.workspace_create(&root.display().to_string(), "Mission Control", &board_env())?;
    let pane = ws.at("root_pane").t("pane_id");
    if pane.is_empty() { crate::fail!("Herdr hat kein Mission-Control-Pane geöffnet.") }
    crate::mission_layout::register(&socket, &pane)?;
    let cmd = format!("{} control", shq(&crate::store::files::bin().display().to_string()));
    api.pane_run(&pane, &cmd)?;
    if focus { api.workspace_focus(ws.at("workspace").s("workspace_id"))?; api.pane_focus(&pane)?; }
    Ok(())
}
pub fn rules(socket: &str) -> String {
    if crate::mission_layout::anchor(socket).is_none() { return String::new() }
    "\n\nMISSION CONTROL: Du bist der dauerhaft sichtbare Orchestrator. Antworte auf Deutsch. Der Mensch wählt links einen lokalen Projektordner und schickt unten einen Task. `board coordinator where` enthält den ausgewählten Ordner für jede Nachricht. Implementiere niemals selbst, editiere keine Projektdateien und übernimm keine Worker-Aufgabe. Du koordinierst und reviewst ausschließlich. Jede MISSION-CONTROL-PROJEKT-Nachricht trägt ihren verbindlichen Projektordner; verwende niemals Tasks anderer Projekte als Bezug. Zerlege den Auftrag in unabhängige Teilaufgaben, lege sie mit `board add ... --repo <vollständiger Ordner> --delegate` an und nutze `--after` für Abhängigkeiten. Lies die AGENTS.md des Projekts und die übergeordnete Developer/AGENTS.md. Prüfe vor Code-Arbeit den Git-Remote; wenn er fehlt, kläre das GitHub-Ziel sichtbar vor dem Start. Ersetze keinen vorhandenen Remote. Verwende für jede Aufgabe die globale worker.agent/worker.fallback-Reihenfolge, wobei die laufende Orchestrator-CLI ausgeschlossen ist; wähle keine anderen Rollen-Modelle. Alle Arbeit läuft in den von Herdr geöffneten echten interaktiven Terminal-Panes. Starte keine unsichtbaren Subagenten, keine ACP-Verbindungen und keine headless/print/exec-Modi. Verstecke Fehler nicht: nenne die Task-ID und die Ursache; die Terminal-Panes bleiben sichtbar. Die Ergebnisse der Teilaufgaben werden dir automatisch in dieser Sitzung zur Prüfung vorgelegt. Wenn eine Nachricht mit [ORCHESTRATOR-REVIEW] beginnt, prüfe selbst den genannten Arbeitsstand und schreibe das verlangte Urteil an den angegebenen Pfad; delegiere dieses Review nicht. Der Workflow verarbeitet dein Urteil und schickt notwendige Korrekturen an den Worker zurück. Berichte dem Menschen Fortschritt, Blockaden und Review-Ergebnisse. Änderungen werden nach erfolgreichen Prüfungen vom Workflow committed und auf den GitHub-Remote gepusht; ein Merge braucht weiterhin die Freigabe des Menschen.\n".into()
}

/// The running coordinator's CLI is reserved for coordination, including after a provider handover.
pub fn worker_allowed(socket: &str, agent: &str) -> bool {
    if crate::mission_layout::anchor(socket).is_none() { return true; }
    let coordinator = state::get(socket).unwrap_or_default();
    let kind = if coordinator.at("run").b("kind") { coordinator.at("run").t("kind") }
        else { crate::mission_fallback::target(socket).map(|r| r.t("kind")).unwrap_or_default() };
    crate::store::settings::values::agent_of(agent).kind != kind
}
pub fn worker_choice(t: &crate::store::tasks::Task, socket: &str) -> Res<String> {
    let w = crate::store::settings::lookup::work(Some(t));
    let mut order = vec![if t.b("on") { t.t("on") } else { w.t("worker.agent") }];
    order.extend(crate::store::settings::values::list_of(w.at("worker.fallback")));
    // The global list still supplies a candidate when the coordinator falls back onto the worker's first CLI.
    order.extend(parse_priority(&priority())?);
    order.into_iter().find(|a| worker_allowed(socket, a) && crate::finish::hand_over::usable(a, w.s("worker.access")).is_empty())
        .ok_or_else(|| crate::err::Error::new("Keine andere verfügbare Worker-CLI. Bitte die CLI-Reihenfolge ergänzen."))
}

/// Tasks added from the coordinator inherit the message's project; a conflicting explicit repo is refused.
pub fn task_folder(folder: &Path, repo: &str, pane: &str, socket: &str) -> Res<PathBuf> {
    let requested = crate::store::repos::folder_for(repo, folder)?;
    if pane.is_empty() || crate::mission_layout::anchor(socket).is_none() { return Ok(requested); }
    let co = state::get(socket).unwrap_or_default();
    if co.s("pane") != pane { return Ok(requested); }
    let Some(message) = co.arr("where").last().filter(|m| m.s("said").starts_with("[MISSION-CONTROL-PROJEKT]")) else { return Ok(requested) };
    let root = PathBuf::from(message.s("root"));
    if !repo.is_empty() && crate::store::repos::primary_root(&requested) != crate::store::repos::primary_root(&root) {
        crate::fail!("Dieser Auftrag gehört zu {}. Ein Task in {} wurde nicht angelegt. Verwende --repo {}.", root.display(), requested.display(), shq(&root.display().to_string()));
    }
    Ok(root)
}
