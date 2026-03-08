use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use url::Url;

use crate::oauth::{OAuthClientConfig, OAuthProvider, OAuthToken, authorize_with_pkce};

const AUTH_URL: &str = "https://ticktick.com/oauth/authorize";
const TOKEN_URL: &str = "https://ticktick.com/oauth/token";
const SCOPES: &[&str] = &["tasks:read", "tasks:write"];

pub struct ApiClient {
    client: Client,
    base_url: Url,
    api_token: OAuthToken,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub status: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectData {
    #[serde(default)]
    pub tasks: Vec<TaskSummary>,
}

struct TickTickOAuthProvider;

impl OAuthProvider for TickTickOAuthProvider {
    fn authorize_url(&self) -> &str {
        AUTH_URL
    }

    fn token_url(&self) -> &str {
        TOKEN_URL
    }

    fn scopes(&self) -> &[&str] {
        SCOPES
    }
}

impl ApiClient {
    pub async fn new(
        base_url: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<Self> {
        let provider = TickTickOAuthProvider;
        let token = authorize_with_pkce(
            &provider,
            OAuthClientConfig {
                client_id,
                client_secret,
                redirect_uri,
            },
        )
        .await?;
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .gzip(true)
            .build()
            .context("failed to build HTTP client")?;
        let normalized_base = if base_url.ends_with('/') {
            base_url.to_string()
        } else {
            format!("{base_url}/")
        };
        let parsed_base = Url::parse(&normalized_base).context("invalid API base URL")?;

        Ok(Self {
            client,
            base_url: parsed_base,
            api_token: token,
        })
    }

    pub async fn get_user_projects(&self) -> Result<Vec<ProjectSummary>> {
        let url = self.base_url.join("project")?;

        let resp = self
            .client
            .get(url)
            .bearer_auth(&self.api_token.access_token)
            .send()
            .await
            .context("failed to send request")?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(anyhow!("request failed: HTTP {status}, body: {body}"));
        }

        serde_json::from_str(&body).context("failed to parse projects response")
    }

    pub async fn get_project_data(&self, project_id: &str) -> Result<ProjectData> {
        let url = self.base_url.join(&format!("project/{project_id}/data"))?;

        let resp = self
            .client
            .get(url)
            .bearer_auth(&self.api_token.access_token)
            .send()
            .await
            .context("failed to send request")?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(anyhow!("request failed: HTTP {status}, body: {body}"));
        }

        serde_json::from_str(&body).context("failed to parse project data response")
    }
}
