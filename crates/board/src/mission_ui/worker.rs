//! Background operations for Mission Control: loading folders, delivering tasks, selecting priority, and live status.
use super::State;
use crate::json::*;
use crate::mission_projects::Project;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, RecvTimeoutError};
use std::time::Duration;

pub enum Command { Refresh, Submit(PathBuf, String), Priority(String) }
pub enum Update { Folders(Vec<Project>), Snapshot(Value), Sent(PathBuf, String), Applied(String), Error(String) }
pub fn start(socket: String, commands: Receiver<Command>, updates: Sender<Update>) {
    std::thread::spawn(move || {
        crate::flow::send::shim();
        crate::extras::refresh_sidebar();
        crate::mission_keeper::ensure();
        refresh(&updates);
        loop {
            match commands.recv_timeout(Duration::from_millis(1000)) {
                Ok(Command::Refresh) => refresh(&updates),
                Ok(Command::Submit(folder, text)) => match crate::mission_control::submit(&folder, &text, &socket) {
                    Ok(()) => { let _ = updates.send(Update::Sent(folder, text)); }
                    Err(e) => { let _ = updates.send(Update::Error(e.msg)); }
                },
                Ok(Command::Priority(text)) => match crate::mission_control::apply_priority(&text) {
                    Ok(primary) => { let _ = crate::coordinator::state::change(socket.as_str(), |s| { s.put("controlPriority", ""); true }); let _ = updates.send(Update::Applied(primary)); }
                    Err(e) => { let _ = updates.send(Update::Error(e.msg)); }
                },
                Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
            if crate::mission_control::has_session(&socket) {
                if let Err(e) = crate::mission_control::ensure(&socket) { let _ = updates.send(Update::Error(e.msg)); }
            }
            crate::mission_keeper::ensure();
            if !crate::keeper::keeper_alive() { crate::keeper::tick_once(&socket, false); }
            let tasks = crate::store::tasks::list(Default::default());
            crate::mission_agents::sync(&tasks, &socket);
            let coordinator = crate::mission_control::selected_summary(socket.as_str());
            if updates.send(Update::Snapshot(coordinator)).is_err() { break; }
        }
    });
}
fn refresh(updates: &Sender<Update>) {
    match crate::mission_projects::scan(&crate::mission_projects::root()) {
        Ok(mut folders) => {
            if let Ok(path) = std::env::var("HERDR_CONTROL_PROJECT_PATH") { folders.retain(|p| p.path == PathBuf::from(&path)); }
            let _ = updates.send(Update::Folders(folders));
        }
        Err(e) => { let _ = updates.send(Update::Error(format!("Projektordner können nicht geladen werden: {e}"))); }
    }
}
pub fn receive(s: &mut State, update: Update) {
    match update {
        Update::Folders(folders) => {
            let wanted = s.folder().map(|p| p.path.display().to_string()).unwrap_or_else(|| crate::mission_layout::state(&s.socket).t("selected"));
            s.save_draft();
            s.projects = folders;
            let index = s.projects.iter().position(|p| p.path.display().to_string() == wanted).unwrap_or(0);
            s.project.select(None); s.choose(index);
            s.notice = format!("{} Projektordner · auswählen, Task eingeben, senden", s.projects.len());
        }
        Update::Snapshot(coordinator) => { s.coordinator = coordinator; }
        Update::Sent(folder, text) => {
            s.sending = false;
            let path = folder.display().to_string();
            if s.folder().map(|p| p.path == folder).unwrap_or(false) && s.draft.text() == text { s.draft.set_text(""); s.save_draft(); }
            else if s.drafts.get(&path).map(|d| d == &text).unwrap_or(false) { s.drafts.remove(&path); }
            s.notice = format!("Task für {} an den Orchestrator übergeben", folder.file_name().unwrap_or_default().to_string_lossy());
        }
        Update::Applied(primary) => { s.priority = false; s.error.clear(); s.notice = format!("Reihenfolge gespeichert und angewendet · Orchestrator: {primary}"); }
        Update::Error(error) => { s.sending = false; s.error = error; }
    }
}
