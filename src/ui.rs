use crate::app::{App, CurrentScreen};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn run(app: &mut App) -> std::io::Result<()> {
    ratatui::run(|terminal| run_app(terminal, app))
}

fn run_app(terminal: &mut DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    while !app.should_exit {
        terminal.draw(|frame| render(frame, app))?;
        handle_events(app)?;
    }

    Ok(())
}

fn handle_events(app: &mut App) -> std::io::Result<()> {
    if let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        match app.current_screen {
            CurrentScreen::ProjectsView => match key.code {
                KeyCode::Up => app.previous_project(),
                KeyCode::Down => app.next_project(),
                KeyCode::Enter => app.open_project_tickets(),
                KeyCode::Char('q') => app.quit(),
                _ => {}
            },
            CurrentScreen::ProjectView => match key.code {
                KeyCode::Up => app.previous_task(),
                KeyCode::Down => app.next_task(),
                KeyCode::Esc => app.back_to_projects(),
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
    }

    let help_text = match app.current_screen {
        CurrentScreen::ProjectsView => "Up/Down: select project | Enter: open tickets | q: quit",
        CurrentScreen::ProjectView => "Up/Down: select ticket | Esc: back | q: quit",
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
                let label = format!("{} ({})", p.project.name, p.tasks.len());
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
