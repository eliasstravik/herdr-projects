//! One project's fixed-space task input and coordinator status.
use super::State;
use crate::board::keys::key_name;
use crate::board::textbox::TextBox;
use crate::json::*;
use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Paragraph}};

fn block(title: impl Into<String>) -> Block<'static> { Block::default().borders(Borders::ALL).title(title.into()).border_style(Style::default().fg(Color::DarkGray)) }
pub fn draw(f: &mut Frame, s: &mut State) {
    let area = f.area();
    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(4), Constraint::Min(6), Constraint::Length(3), Constraint::Length(3),
    ]).split(area);
    let actual = if s.coordinator.b("agent") { s.coordinator.s("agent") } else { "wird gestartet" };
    let status = match s.coordinator.s("state") { "none" => "noch nicht gestartet", "starting" => "startet", "working" => "arbeitet", "blocked" => "wartet auf Antwort", "idle" => "bereit", "exited" => "beendet", other => other };
    let queued = s.coordinator.i("queued");
    let selected = crate::mission_control::priority().split(',').next().unwrap_or("").to_string();
    let header = vec![
        Line::from(Span::styled("MISSION CONTROL", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(format!("Orchestrator: {actual} · {status} · {queued} warten   |   Priorität 1: {selected}")),
        Line::from(format!("Space: {} · Orchestrator und Worker laufen als sichtbare Terminal-Panes", s.folder().map(|p| p.name.as_str()).unwrap_or("Projekt"))),
    ];
    f.render_widget(Paragraph::new(header), chunks[0]);
    s.folders_rect = Rect::default();
    let project = s.folder().cloned();
    let title = if s.priority { "Globale CLI-Reihenfolge · erste CLI = Orchestrator".into() } else { format!("Task für {}", project.as_ref().map(|p| p.name.as_str()).unwrap_or("Projekt")) };
    let input_block = block(title).border_style(Style::default().fg(if s.sidebar && !s.priority { Color::DarkGray } else { Color::Cyan }));
    s.input_rect = input_block.inner(chunks[1]);
    f.render_widget(input_block, chunks[1]);
    let rect = s.input_rect;
    let focused = true;
    if s.priority { editor(f, &mut s.order, rect, true); } else { editor(f, &mut s.draft, rect, focused); }
    let buttons = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Length(22), Constraint::Min(10)]).split(chunks[2]);
    s.send_rect = buttons[0]; s.priority_rect = buttons[1]; s.refresh_rect = Rect::default();
    let send = if s.priority { "Reihenfolge anwenden" } else if s.sending { "Wird übergeben …" } else { "Task senden ↵" };
    for (rect, text) in [(buttons[0], send), (buttons[1], "CLI-Reihenfolge")] { f.render_widget(Paragraph::new(text).block(block("")).style(Style::default().fg(Color::Cyan)), rect); }
    let sync = project.as_ref().map(|p| if p.remote.is_empty() { "Lokaler Ordner · kein Git-Remote; GitHub-Sync erst nach Repository-Einrichtung".into() } else { format!("Git-Sync nach erfolgreicher Prüfung: {}", p.remote) }).unwrap_or_default();
    let note = if !s.error.is_empty() { s.error.clone() } else { s.notice.clone() };
    let tip = format!("{} Task senden · {} neue Zeile · {} Reihenfolge", key_name("enter"), key_name("newline"), key_name("settings"));
    let foot = vec![Line::from(Span::styled(note, Style::default().fg(if s.error.is_empty() { Color::Gray } else { Color::Red }))), Line::from(sync), Line::from(tip)];
    f.render_widget(Paragraph::new(foot), chunks[3]);
}
fn editor(f: &mut Frame, box_: &mut TextBox, rect: Rect, focus: bool) {
    let width = rect.width.saturating_sub(1).max(1) as usize;
    box_.layout_width = width;
    let visual = box_.visual(width);
    let at = box_.cursor_visual(&visual);
    let height = rect.height.max(1) as usize;
    if at < box_.viewport_top { box_.viewport_top = at; }
    if at >= box_.viewport_top + height { box_.viewport_top = at + 1 - height; }
    let text = box_.text();
    let source: Vec<&str> = text.split('\n').collect();
    let lines: Vec<Line> = visual.iter().skip(box_.viewport_top).take(height).map(|(n, chunk)| Line::from(source.get(*n).unwrap_or(&"")[chunk.start..chunk.end].to_string())).collect();
    f.render_widget(Paragraph::new(lines), rect);
    if focus && rect.height > 0 && rect.width > 0 {
        let (n, chunk) = &visual[at];
        let line = source.get(*n).unwrap_or(&"");
        let col = box_.cursor().1.clamp(chunk.start, chunk.end);
        let x = crate::text::width(&line[chunk.start..col]) as u16;
        f.set_cursor_position((rect.x + x.min(rect.width - 1), rect.y + at.saturating_sub(box_.viewport_top) as u16));
    }
}
