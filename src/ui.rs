use crate::app::{App, CurrentScreen};
use crate::ticktick::ApiClient;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn run(
    app: &mut App,
    client: &ApiClient,
    runtime: &tokio::runtime::Runtime,
) -> std::io::Result<()> {
    ratatui::run(|terminal| run_app(terminal, app, client, runtime))
}

fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    client: &ApiClient,
    runtime: &tokio::runtime::Runtime,
) -> std::io::Result<()> {
    while !app.should_exit {
        terminal.draw(|frame| render(frame, app))?;
        runtime.block_on(handle_events(app, client))?;
    }

    Ok(())
}

fn non_empty_or_dash(value: Option<&str>) -> &str {
    match value {
        Some(v) if !v.trim().is_empty() => v,
        _ => "-",
    }
}

fn status_value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::Bool(v) => {
            if *v {
                "done".to_string()
            } else {
                "open".to_string()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            if s.trim().is_empty() {
                "-".to_string()
            } else {
                s.clone()
            }
        }
        _ => value.to_string(),
    }
}

fn item_is_done(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(v) => *v,
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
        serde_json::Value::String(s) => {
            let normalized = s.trim().to_lowercase();
            normalized == "1" || normalized == "true" || normalized == "done"
        }
        _ => false,
    }
}

async fn handle_events(app: &mut App, client: &ApiClient) -> std::io::Result<()> {
    if let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        match app.current_screen {
            CurrentScreen::ProjectsView => match key.code {
                KeyCode::Up => app.previous_project(),
                KeyCode::Down => app.next_project(),
                KeyCode::Enter => app.open_project_tickets(client).await,
                KeyCode::Char('q') => app.quit(),
                _ => {}
            },
            CurrentScreen::ProjectView => match key.code {
                KeyCode::Up => app.previous_task(),
                KeyCode::Down => app.next_task(),
                KeyCode::Enter => app.open_ticket_details(client).await,
                KeyCode::Esc => app.back(),
                KeyCode::Char('q') => app.quit(),
                _ => {}
            },
            CurrentScreen::TicketView => match key.code {
                KeyCode::Esc => app.back(),
                KeyCode::Char('q') => app.quit(),
                _ => {}
            },
        }
    }

    Ok(())
}

fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(frame.area());
    match app.current_screen {
        CurrentScreen::ProjectsView => render_projects_view(frame, app, chunks[0]),
        CurrentScreen::ProjectView => render_project_tickets_view(frame, app, chunks[0]),
        CurrentScreen::TicketView => render_ticket_view(frame, app, chunks[0]),
    }

    let help_text = match app.current_screen {
        CurrentScreen::ProjectsView => "Up/Down: select project | Enter: open tickets | q: quit",
        CurrentScreen::ProjectView => {
            "Up/Down: select ticket | Enter: open details | Esc: back | q: quit"
        }
        CurrentScreen::TicketView => "Esc: back to tickets | q: quit",
    };
    frame.render_widget(
        Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}

fn render_projects_view(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = if app.projects.is_empty() {
        vec![ListItem::new("No projects found")]
    } else {
        app.projects
            .iter()
            .map(|p| {
                let label = p.project.name.to_string();
                ListItem::new(label)
            })
            .collect()
    };

    let mut list_state = ListState::default();
    if !app.projects.is_empty() {
        list_state.select(Some(app.selected_project));
    }

    let list = List::new(items)
        .block(Block::default().title("Projects").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold())
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_project_tickets_view(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(err) = app.last_error.as_ref() {
        frame.render_widget(
            Paragraph::new(err.as_str())
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .title("Project Tickets")
                        .borders(Borders::ALL),
                ),
            area,
        );
        return;
    }

    let Some(project) = app.selected_project() else {
        frame.render_widget(
            Paragraph::new("No project selected").block(
                Block::default()
                    .title("Project Tickets")
                    .borders(Borders::ALL),
            ),
            area,
        );
        return;
    };

    let items: Vec<ListItem> = if project.tasks.is_empty() {
        vec![ListItem::new("No tickets in this project")]
    } else {
        project
            .tasks
            .iter()
            .map(|task| {
                let status = if task.status == 0 { "open" } else { "done" };
                ListItem::new(format!("{} [{}]", task.title, status))
            })
            .collect()
    };

    let mut list_state = ListState::default();
    if !project.tasks.is_empty() {
        list_state.select(Some(app.selected_task));
    }

    let title = format!("Tickets: {}", project.project.name);
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Green).bold())
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_ticket_view(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(err) = app.last_error.as_ref() {
        frame.render_widget(
            Paragraph::new(err.as_str())
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .title("Ticket Details")
                        .borders(Borders::ALL),
                ),
            area,
        );
        return;
    }

    let Some(task) = app.viewed_ticket.as_ref() else {
        frame.render_widget(
            Paragraph::new("Ticket details are not available for this task").block(
                Block::default()
                    .title("Ticket Details")
                    .borders(Borders::ALL),
            ),
            area,
        );
        return;
    };

    let content = non_empty_or_dash(task.content.as_deref());
    let desc = non_empty_or_dash(task.desc.as_deref());
    let start_date = non_empty_or_dash(task.start_date.as_deref());
    let end_date = non_empty_or_dash(task.end_date.as_deref());
    let completed_time = non_empty_or_dash(task.completed_time.as_deref());
    let status = status_value_text(&task.status);

    let items_text = if task.items.is_empty() {
        "  (none)".to_string()
    } else {
        task.items
            .iter()
            .map(|item| {
                let mark = if item_is_done(&item.status) { "x" } else { " " };
                let done_at = match item.completed_time.as_deref() {
                    Some(value) if !value.trim().is_empty() => format!(" | completed: {value}"),
                    _ => String::new(),
                };
                format!("  - [{}] {} (id: {}){}", mark, item.title, item.id, done_at)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let body = format!(
        "ID: {}\nStatus: {}\nStart: {}\nEnd: {}\nCompleted: {}\n\nTitle:\n{}\n\nContent:\n{}\n\nDescription:\n{}\n\nItems:\n{}",
        task.id,
        status,
        start_date,
        end_date,
        completed_time,
        task.title,
        content,
        desc,
        items_text
    );

    let paragraph = Paragraph::new(body)
        .block(
            Block::default()
                .title("Ticket Details")
                .borders(Borders::ALL),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
