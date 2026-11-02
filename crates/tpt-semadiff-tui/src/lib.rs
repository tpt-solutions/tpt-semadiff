//! Interactive terminal viewer for a [`tpt_semadiff_core::report::Report`].
//!
//! Left pane: one entry per function that changed. Right pane: its changes.
//! Keys: `j`/`k` or the arrow keys move, `q` or `Esc` quits.

use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use tpt_semadiff_core::report::{Group, OpKind, Report};

/// Runs the viewer until the user quits.
pub fn run(report: &Report) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, report);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, report: &Report) -> io::Result<()> {
    let mut state = ListState::default();
    if !report.groups.is_empty() {
        state.select(Some(0));
    }
    loop {
        terminal.draw(|frame| draw(frame, report, &mut state))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let last = report.groups.len().saturating_sub(1);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Char('j') | KeyCode::Down => {
                state.select(Some(state.selected().map_or(0, |i| (i + 1).min(last))));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                state.select(Some(state.selected().map_or(0, |i| i.saturating_sub(1))));
            }
            _ => {}
        }
    }
}

/// Draws the two-pane view. Public so the layout can be rendered into a test backend.
pub fn draw(frame: &mut Frame, report: &Report, state: &mut ListState) {
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
            .areas(frame.area());

    let items: Vec<ListItem> = report
        .groups
        .iter()
        .map(|g| ListItem::new(group_title(g)))
        .collect();
    let list = List::new(items)
        .block(Block::bordered().title(format!("{} ({} changes)", report.new_name, total(report))))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, list_area, state);

    let selected = state.selected().and_then(|i| report.groups.get(i));
    let (title, lines) = match selected {
        Some(group) => (
            format!("Lines {}-{}", group.start_line, group.end_line),
            detail_lines(group),
        ),
        None => (String::new(), vec![Line::from("No semantic changes.")]),
    };
    let detail = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(Block::bordered().title(title));
    frame.render_widget(detail, detail_area);
}

fn total(report: &Report) -> usize {
    let c = report.counts;
    c.insert + c.delete + c.moved + c.update
}

fn group_title(group: &Group) -> String {
    group
        .function
        .as_deref()
        .map(|name| format!("fn {name}"))
        .unwrap_or_else(|| "Top level".to_owned())
}

fn detail_lines(group: &Group) -> Vec<Line<'static>> {
    group
        .changes
        .iter()
        .map(|change| {
            let (label, color) = match change.op {
                OpKind::Insert => ("INSERT", Color::Green),
                OpKind::Delete => ("DELETE", Color::Red),
                OpKind::Move => ("MOVED", Color::Yellow),
                OpKind::Update => ("UPDATE", Color::Cyan),
            };
            Line::from(vec![
                Span::styled(format!("[{label}]"), Style::new().fg(color)),
                Span::raw(format!(" {}", change.description)),
            ])
        })
        .collect()
}
