use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;
use url::Url;

#[derive(Debug, Deserialize, Clone)]
pub struct OAuthToken {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub scope: Option<String>,
}

pub struct OAuthClientConfig<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub redirect_uri: &'a str,
}

pub trait OAuthProvider {
    fn authorize_url(&self) -> &str;
    fn token_url(&self) -> &str;
    fn scopes(&self) -> &[&str];

    fn authorize_extra_params(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    fn token_extra_params(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}

#[derive(Debug, Clone)]
struct AppState {
    expected_state: String,
    tx_code: Arc<Mutex<Option<oneshot::Sender<String>>>>,
    tx_shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

fn gen_state() -> String {
    let mut b = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut b);
    hex::encode(b)
}

fn gen_pkce() -> (String, String) {
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    let verifier = URL_SAFE_NO_PAD.encode(b);

    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(digest);

    (verifier, challenge)
}

async fn callback_handler(
    State(st): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<&'static str> {
    if let Some(err) = params.get("error") {
        eprintln!("OAuth error: {err}");
        if let Some(tx) = st.tx_shutdown.lock().unwrap().take() {
            let _ = tx.send(());
        }
        return Html("Authorization failed (error param). You can close this tab.");
    }

    let code = params.get("code").cloned();
    let returned_state = params.get("state").cloned();

    if returned_state.as_deref() != Some(&st.expected_state) {
        if let Some(tx) = st.tx_shutdown.lock().unwrap().take() {
            let _ = tx.send(());
        }
        return Html("Invalid state. You can close this tab.");
    }

    match code {
        Some(code) => {
            if let Some(tx) = st.tx_code.lock().unwrap().take() {
                let _ = tx.send(code);
            }
            if let Some(tx) = st.tx_shutdown.lock().unwrap().take() {
                let _ = tx.send(());
            }
            Html("Authorization successful! You can close this tab and return to the CLI.")
        }
        None => {
            if let Some(tx) = st.tx_shutdown.lock().unwrap().take() {
                let _ = tx.send(());
            }
            Html("Missing code. You can close this tab.")
        }
    }
}

fn build_authorize_url<P: OAuthProvider>(
    provider: &P,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
) -> Result<Url> {
    let mut url = Url::parse(provider.authorize_url())?;
    let scope = provider.scopes().join(" ");
    let mut query = url.query_pairs_mut();
    query
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");

    if !scope.is_empty() {
        query.append_pair("scope", &scope);
    }

    for (key, value) in provider.authorize_extra_params() {
        query.append_pair(&key, &value);
    }
    drop(query);

    Ok(url)
}

fn parse_token_response(body: &str) -> Result<OAuthToken> {
    if let Ok(v) = serde_json::from_str::<OAuthToken>(body) {
        return Ok(v);
    }
    let v = serde_urlencoded::from_str::<OAuthToken>(body).map_err(|e| {
        anyhow!("Failed to parse token response as JSON or form-encoded: {e}. Body: {body}")
    })?;
    Ok(v)
}

async fn exchange_code_for_token<P: OAuthProvider>(
    provider: &P,
    http: &Client,
    cfg: &OAuthClientConfig<'_>,
    code: &str,
    code_verifier: &str,
) -> Result<OAuthToken> {
    let mut params = vec![
        ("grant_type".to_string(), "authorization_code".to_string()),
        ("client_id".to_string(), cfg.client_id.to_string()),
        ("client_secret".to_string(), cfg.client_secret.to_string()),
        ("code".to_string(), code.to_string()),
        ("redirect_uri".to_string(), cfg.redirect_uri.to_string()),
        ("code_verifier".to_string(), code_verifier.to_string()),
    ];
    params.extend(provider.token_extra_params());

    let resp = http
        .post(provider.token_url())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await
        .context("token request failed")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!(
            "Token exchange failed: HTTP {status}, body: {body}"
        ));
    }

    parse_token_response(&body)
}

pub async fn authorize_with_pkce<P: OAuthProvider>(
    provider: &P,
    cfg: OAuthClientConfig<'_>,
) -> Result<OAuthToken> {
    let (code_verifier, code_challenge) = gen_pkce();
    let state = gen_state();

    let auth_url = build_authorize_url(
        provider,
        cfg.client_id,
        cfg.redirect_uri,
        &state,
        &code_challenge,
    )?;
    eprintln!("Opening browser for authorization...");
    eprintln!("If it doesn't open, copy/paste this URL:\n{auth_url}\n");

    let redirect = Url::parse(cfg.redirect_uri)?;
    let port = redirect.port().unwrap_or(8080);
    let path = redirect.path().to_string();

    let (tx_code, rx_code) = oneshot::channel::<String>();
    let (tx_shutdown, rx_shutdown) = oneshot::channel::<()>();

    let st = AppState {
        expected_state: state,
        tx_code: Arc::new(Mutex::new(Some(tx_code))),
        tx_shutdown: Arc::new(Mutex::new(Some(tx_shutdown))),
    };

    let app = Router::new()
        .route(&path, get(callback_handler))
        .with_state(st);

    let addr: SocketAddr = format!("127.0.0.1:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {addr} (is the port in use?)"))?;

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = rx_shutdown.await;
            })
            .await
            .ok();
    });

    if let Err(err) = open::that(auth_url.as_str()) {
        eprintln!("Failed to auto-open browser: {err}");
    }

    let code = rx_code
        .await
        .context("did not receive authorization code")?;

    let _ = server_task.await;

    let http = Client::new();
    exchange_code_for_token(provider, &http, &cfg, &code, &code_verifier).await
}
