//! Keyboard and mouse actions in Mission Control, using the board's existing key definitions and editor.
use super::{State, worker::Command};
use crate::board::keys::{self, K};
use crossterm::event::{Event, MouseEventKind, MouseButton};
use std::time::{Duration, Instant};

pub fn handle(s: &mut State, event: Event) -> bool {
    match event {
        Event::Paste(text) => { if s.priority { s.order.insert(&text); } else { s.sidebar = false; s.draft.insert(&text); } }
        Event::Key(key) => {
            let Some(p) = crate::board::run::input::press_of(&key) else { return false };
            if keys::is(&p, K::Close) {
                if s.closed_at.map(|at| at.elapsed() < Duration::from_secs(2)).unwrap_or(false) { return true; }
                s.closed_at = Some(Instant::now()); s.notice = "Erneut schließen: Mission Control schließen; laufende Agenten bleiben erhalten".into(); return false;
            }
            if keys::is(&p, K::Settings) { priority(s); return false; }
            if s.priority {
                if keys::is(&p, K::Back) { s.priority = false; }
                else if keys::is(&p, K::Enter) { let _ = s.tx.send(Command::Priority(s.order.text())); }
                else { s.order.handle(&p); }
                return false;
            }
            if keys::is(&p, K::Delegate) { return false; }
            if keys::is(&p, K::Enter) { s.send(); }
            else { s.draft.handle(&p); }
        }
        Event::Mouse(m) => {
            let point = ratatui::layout::Position::new(m.column, m.row);
            match m.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if s.send_rect.contains(point) { if s.priority { let _ = s.tx.send(Command::Priority(s.order.text())); } else { s.send(); } }
                    else if s.priority_rect.contains(point) { priority(s); }
                    else if s.input_rect.contains(point) { }
                }
                _ => {}
            }
        }
        _ => {}
    }
    false
}
fn priority(s: &mut State) { s.priority = !s.priority; if s.priority { s.order.set_text(&crate::mission_control::priority()); } }
