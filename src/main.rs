use anyhow::{Context, Result};
use app::{App, ProjectWithTasks};
use ticktick::ApiClient;

pub mod app;
pub mod oauth;
pub mod ticktick;
pub mod ui;

const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8080/callback";

fn required_env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("missing required environment variable: {name}"))
}

fn main() -> Result<()> {
    let _ = color_eyre::install();
    let runtime = tokio::runtime::Runtime::new().context("failed to create Tokio runtime")?;

    let client_id = required_env("TICKTICK_CLIENT_ID")?;
    let client_secret = required_env("TICKTICK_CLIENT_SECRET")?;
    let redirect_uri =
        std::env::var("TICKTICK_REDIRECT_URI").unwrap_or_else(|_| DEFAULT_REDIRECT_URI.to_string());

    let client = runtime.block_on(ApiClient::new(&client_id, &client_secret, &redirect_uri))?;
    let projects = runtime.block_on(client.get_user_projects())?;
    let mut project_rows = Vec::with_capacity(projects.len());
    for project in projects {
        let tasks = Vec::new();
        project_rows.push(ProjectWithTasks { project, tasks });
    }

    let mut app = App::new(project_rows);
    ui::run(&mut app, &client, &runtime)?;

    Ok(())
}
