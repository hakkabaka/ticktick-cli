use anyhow::{Context, Result};
use app::{App, ProjectWithTasks};
use ticktick::ApiClient;

pub mod app;
pub mod oauth;
pub mod ticktick;
pub mod ui;

const BASE_URL: &str = "https://api.ticktick.com/open/v1";
const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8080/callback";

fn required_env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("missing required environment variable: {name}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = color_eyre::install();

    let client_id = required_env("TICKTICK_CLIENT_ID")?;
    let client_secret = required_env("TICKTICK_CLIENT_SECRET")?;
    let redirect_uri =
        std::env::var("TICKTICK_REDIRECT_URI").unwrap_or_else(|_| DEFAULT_REDIRECT_URI.to_string());

    let client = ApiClient::new(BASE_URL, &client_id, &client_secret, &redirect_uri).await?;
    let projects = client.get_user_projects().await?;
    let mut project_rows = Vec::with_capacity(projects.len());
    for project in projects {
        let tasks = match client.get_project_data(&project.id).await {
            Ok(data) => data.tasks,
            Err(_) => Vec::new(),
        };
        project_rows.push(ProjectWithTasks { project, tasks });
    }

    let mut app = App::new(project_rows);
    ui::run(&mut app)?;

    Ok(())
}
