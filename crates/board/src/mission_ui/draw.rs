//! Mission Control's project sidebar, task overview, orchestrator status, and bottom task input with clickable actions.
use super::State;
use crate::board::keys::key_name;
use crate::board::textbox::TextBox;
use crate::json::*;
use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, List, ListItem, Paragraph, Wrap}};

fn block(title: impl Into<String>) -> Block<'static> { Block::default().borders(Borders::ALL).title(title.into()).border_style(Style::default().fg(Color::DarkGray)) }
pub fn draw(f: &mut Frame, s: &mut State) {
    let area = f.area();
    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(4), Constraint::Min(6), Constraint::Length(8), Constraint::Length(3), Constraint::Length(3),
    ]).split(area);
    let actual = if s.coordinator.b("agent") { s.coordinator.s("agent") } else { "wird gestartet" };
    let status = match s.coordinator.s("state") { "none" => "noch nicht gestartet", "starting" => "startet", "working" => "arbeitet", "blocked" => "wartet auf Antwort", "idle" => "bereit", "exited" => "beendet", other => other };
    let queued = s.coordinator.i("queued");
    let selected = crate::mission_control::priority().split(',').next().unwrap_or("").to_string();
    let header = vec![
        Line::from(Span::styled("MISSION CONTROL", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(format!("Orchestrator: {actual} · {status} · {queued} warten   |   Priorität 1: {selected}")),
        Line::from("Projekt links auswählen · Task unten senden · Live-Terminals rechts"),
    ];
    f.render_widget(Paragraph::new(header), chunks[0]);
    let columns = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(40), Constraint::Percentage(60)]).split(chunks[1]);
    let folders = block("Developer · Projekte");
    s.folders_rect = folders.inner(columns[0]);
    let items: Vec<ListItem> = s.projects.iter().map(|p| {
        ListItem::new(format!("{} {}", if p.remote.is_empty() { "·" } else { "↑" }, p.name))
    }).collect();
    f.render_stateful_widget(List::new(items).block(folders).highlight_style(Style::default().bg(Color::Blue).fg(Color::White)).highlight_symbol("› "), columns[0], &mut s.project);
    let project = s.folder().cloned();
    let rows: Vec<ListItem> = s.tasks.iter().filter(|t| project.as_ref().map(|p| crate::store::repos::folder_of(t.s("id")) == p.path).unwrap_or(false)).map(|t| {
        let color = match t.s("stage") { "failed" | "blocked" | "reviewneeds" => Color::Red, "done" => Color::Green, "approval" => Color::Yellow, _ => Color::Cyan };
        ListItem::new(vec![Line::from(vec![Span::styled(format!("{} · {}", t.s("id"), t.s("stage")), Style::default().fg(color))]),
            Line::from(t.t("title")), Line::from(Span::styled(t.t("msg"), Style::default().fg(Color::Gray)))])
    }).collect();
    let task_parts = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(65), Constraint::Percentage(35)]).split(columns[1]);
    s.tasks_rect = task_parts[0];
    if rows.is_empty() {
        f.render_widget(Paragraph::new("Noch keine Tasks in diesem Projekt.\n\nTask unten eingeben. Der Orchestrator verteilt Teilaufgaben und prüft die Ergebnisse.").wrap(Wrap { trim: false }).block(block("Tasks")), task_parts[0]);
    } else { f.render_stateful_widget(List::new(rows).block(block("Tasks · Fortschritt / Fehler")), task_parts[0], &mut s.task); }
    let said = if s.coordinator.b("error") { s.coordinator.t("error") } else if s.coordinator.b("said") { s.coordinator.t("said") } else { "Die komplette Orchestrator-Sitzung ist im Terminal rechts sichtbar.".into() };
    f.render_widget(Paragraph::new(said).wrap(Wrap { trim: false }).block(block("Orchestrator · Rückmeldung")), task_parts[1]);
    let title = if s.priority { "Globale CLI-Reihenfolge · erste CLI = Orchestrator".into() } else { format!("Task für {}", project.as_ref().map(|p| p.name.as_str()).unwrap_or("Projekt auswählen")) };
    let input_block = block(title).border_style(Style::default().fg(if s.sidebar && !s.priority { Color::DarkGray } else { Color::Cyan }));
    s.input_rect = input_block.inner(chunks[2]);
    f.render_widget(input_block, chunks[2]);
    let rect = s.input_rect;
    let focused = !s.sidebar || s.priority;
    if s.priority { editor(f, &mut s.order, rect, true); } else { editor(f, &mut s.draft, rect, focused); }
    let buttons = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Length(22), Constraint::Length(24), Constraint::Min(10)]).split(chunks[3]);
    s.send_rect = buttons[0]; s.priority_rect = buttons[1]; s.refresh_rect = buttons[2];
    let send = if s.priority { "Reihenfolge anwenden" } else if s.sending { "Wird übergeben …" } else { "Task senden ↵" };
    for (rect, text) in [(buttons[0], send), (buttons[1], "CLI-Reihenfolge"), (buttons[2], "Ordner neu laden")] {
        f.render_widget(Paragraph::new(text).block(block("")).style(Style::default().fg(Color::Cyan)), rect);
    }
    let sync = project.as_ref().map(|p| if p.remote.is_empty() { "Lokaler Ordner · kein Git-Remote; GitHub-Sync erst nach Repository-Einrichtung".into() } else { format!("Git-Sync nach erfolgreicher Prüfung: {}", p.remote) }).unwrap_or_default();
    let note = if !s.error.is_empty() { s.error.clone() } else { s.notice.clone() };
    let tip = format!("{} senden · {} neue Zeile · {} Projekt/Eingabe · {} Reihenfolge", key_name("enter"), key_name("newline"), key_name("delegate"), key_name("settings"));
    let foot = vec![Line::from(Span::styled(note, Style::default().fg(if s.error.is_empty() { Color::Gray } else { Color::Red }))), Line::from(sync), Line::from(tip)];
    f.render_widget(Paragraph::new(foot), chunks[4]);
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
