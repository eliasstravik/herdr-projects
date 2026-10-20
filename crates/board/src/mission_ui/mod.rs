//! A project's own Herdr Space: fixed project identity, persistent task composer, and visible CLI panes.
pub mod draw;
pub mod events;
pub mod worker;
use crate::board::textbox::TextBox;
use crate::json::*;
use crate::mission_projects::Project;
use crossterm::event;
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect, widgets::ListState};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};

pub struct State {
    pub projects: Vec<Project>, pub project: ListState,
    pub draft: TextBox, pub order: TextBox, pub priority: bool, pub sidebar: bool, pub sending: bool,
    pub notice: String, pub error: String, pub coordinator: Value, pub drafts: HashMap<String, String>,
    pub folders_rect: Rect, pub input_rect: Rect, pub send_rect: Rect, pub priority_rect: Rect, pub refresh_rect: Rect,
    pub socket: String, pub tx: Sender<worker::Command>, pub closed_at: Option<Instant>,
}
impl State {
    pub fn folder(&self) -> Option<&Project> { self.projects.get(self.project.selected().unwrap_or(0)) }
    pub fn save_draft(&mut self) {
        if let Some(p) = self.project.selected().and_then(|i| self.projects.get(i)) {
            let path = p.path.display().to_string(); let value = self.draft.text(); self.drafts.insert(path.clone(), value.clone());
            let _ = crate::store::files::with_lock_wait("mission-control-drafts", 5000, || {
                let mut drafts: HashMap<String, String> = crate::store::files::read_json(&draft_file()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
                drafts.insert(path, value); crate::store::files::write_json(&draft_file(), &json!(drafts))
            });
        }
    }
    pub fn choose(&mut self, index: usize) {
        if self.projects.is_empty() { return; }
        self.save_draft(); self.project.select(Some(index.min(self.projects.len() - 1)));
        if let Some(p) = self.folder() {
            let path = p.path.display().to_string();
            self.draft.set_text(self.drafts.get(&path).map(String::as_str).unwrap_or(""));
            if std::env::var_os("HERDR_CONTROL_PROJECT_PATH").is_none() { let _ = crate::mission_layout::selected(&self.socket, &path); }
        }
    }
    pub fn send(&mut self) {
        if self.sending || self.draft.is_empty() { return; }
        if let Some(p) = self.folder() {
            let folder = p.path.clone(); let text = self.draft.text();
            self.save_draft(); self.sending = true; self.notice = "Task wird an den Orchestrator übergeben …".into(); self.error.clear();
            let _ = self.tx.send(worker::Command::Submit(folder, text));
        }
    }
}
fn draft_file() -> std::path::PathBuf { crate::store::files::data_dir().join("mission-control-drafts.json") }
struct Screen;
impl Drop for Screen { fn drop(&mut self) { crate::board::run::term::leave(false); } }
pub fn open() -> crate::err::Res<i32> {
    let socket = crate::herdr::cli::current_socket();
    let pane = std::env::var("HERDR_PANE_ID").unwrap_or_default();
    if pane.is_empty() { crate::fail!("Mission Control muss in einem echten Herdr-Pane laufen.") }
    let pinned = std::env::var("HERDR_CONTROL_PROJECT_PATH").ok().filter(|p| std::path::Path::new(p).is_dir());
    if let Some(folder) = &pinned {
        let current = crate::herdr::Herdr::new(Some(socket.clone())).pane_get(&pane).unwrap_or_default();
        crate::mission_layout::register_project(&socket, folder, current.s("workspace_id"), &pane)?;
    } else { crate::mission_layout::register(&socket, &pane)?; }
    let drafts: HashMap<String, String> = crate::store::files::read_json(&draft_file()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    let (tx, commands) = channel(); let (updates, rx) = channel();
    worker::start(socket.clone(), commands, updates);
    crate::board::run::term::enter()?;
    let _screen = Screen;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let mut s = State {
        projects: vec![], project: ListState::default(), draft: TextBox::default(), order: TextBox::default(),
        priority: false, sidebar: true, sending: false, notice: "Projektordner werden geladen …".into(), error: String::new(), coordinator: Value::Null, drafts,
        folders_rect: Rect::default(), input_rect: Rect::default(), send_rect: Rect::default(), priority_rect: Rect::default(), refresh_rect: Rect::default(),
        socket, tx, closed_at: None,
    };
    if let Some(path) = pinned {
        if let Ok(mut folders) = crate::mission_projects::scan(std::path::Path::new(&path).parent().unwrap_or(std::path::Path::new(&path))) {
            folders.retain(|p| p.path == std::path::Path::new(&path)); s.projects = folders;
        }
        if s.projects.is_empty() { s.projects.push(Project { name: std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy().into_owned(), path: path.clone().into(), remote: String::new() }); }
        s.project.select(Some(0));
    }
    loop {
        for update in rx.try_iter() { worker::receive(&mut s, update); }
        terminal.draw(|frame| draw::draw(frame, &mut s))?;
        if event::poll(Duration::from_millis(100))? && events::handle(&mut s, event::read()?) { break; }
    }
    s.save_draft();
    Ok(0)
}
