use anyhow::{anyhow, Context, Result};
use rspotify::{prelude::*, scopes, AuthCodePkceSpotify, Credentials, OAuth, Token};
use std::{fs, path::PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn credentials_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Could not determine config directory"))?;
    Ok(config_dir.join("spotify-dj").join("credentials.json"))
}

pub fn save_token(token: &Token) -> Result<()> {
    let path = credentials_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create config directory")?;
    }
    let json = serde_json::to_string_pretty(token).context("Failed to serialize token")?;
    fs::write(&path, json).context("Failed to write credentials file")?;
    Ok(())
}

pub fn load_token() -> Option<Token> {
    let path = credentials_path().ok()?;
    let contents = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn build_oauth() -> OAuth {
    OAuth {
        redirect_uri: "http://127.0.0.1:8888/callback".to_string(),
        scopes: scopes!(
            "user-read-playback-state",
            "user-modify-playback-state",
            "user-read-currently-playing"
        ),
        ..Default::default()
    }
}

pub async fn run_auth_flow(client_id: &str) -> Result<()> {
    let creds = Credentials::new_pkce(client_id);
    let oauth = build_oauth();
    let mut spotify = AuthCodePkceSpotify::new(creds, oauth);

    let url = spotify
        .get_authorize_url(None)
        .context("Failed to generate authorization URL")?;

    eprintln!("Opening Spotify login page in your browser...");
    if let Err(e) = open::that(&url) {
        eprintln!("Could not open browser automatically: {}", e);
        eprintln!("Please open this URL manually:");
        eprintln!("{}", url);
    }

    eprintln!("Waiting for authentication on http://127.0.0.1:8888/callback ...");
    let code = wait_for_callback().await?;

    spotify
        .request_token(&code)
        .await
        .context("Failed to exchange authorization code for token")?;

    let guard = spotify
        .token
        .lock()
        .await
        .map_err(|_| anyhow!("Failed to acquire token lock"))?;
    let token = (*guard)
        .clone()
        .ok_or_else(|| anyhow!("No token received after authentication"))?;
    drop(guard);

    save_token(&token).context("Failed to save credentials")?;
    crate::output::print_json(&serde_json::json!({"authenticated": true}));
    Ok(())
}

async fn wait_for_callback() -> Result<String> {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:8888")
        .await
        .context("Failed to bind to port 8888 — is something else already using it?")?;

    let (mut socket, _) = listener
        .accept()
        .await
        .context("Failed to accept connection on port 8888")?;

    let mut buf = vec![0u8; 4096];
    let n = socket
        .read(&mut buf)
        .await
        .context("Failed to read HTTP request from socket")?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let response = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/html\r\n",
        "Connection: close\r\n",
        "\r\n",
        "<html><body>",
        "<h1>Authentication successful!</h1>",
        "<p>You can close this window and return to your terminal.</p>",
        "</body></html>"
    );
    let _ = socket.write_all(response.as_bytes()).await;

    // Parse the first request line: GET /callback?code=xxx&state=yyy HTTP/1.1
    let first_line = request
        .lines()
        .next()
        .ok_or_else(|| anyhow!("Empty HTTP request received"))?;

    let path = first_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("Malformed HTTP request line"))?;

    if path.contains("error=") {
        let error = extract_param(path, "error").unwrap_or("unknown");
        return Err(anyhow!("Spotify OAuth error: {}", error));
    }

    extract_param(path, "code")
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("No authorization code in callback URL"))
}

fn extract_param<'a>(path: &'a str, param: &str) -> Option<&'a str> {
    let query = path.split('?').nth(1)?;
    let search = format!("{}=", param);
    for part in query.split('&') {
        if part.starts_with(&search) {
            return Some(&part[search.len()..]);
        }
    }
    None
}

pub async fn auth_status() -> Result<()> {
    match load_token() {
        Some(token) => {
            let expires_at = token
                .expires_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "unknown".to_string());
            crate::output::print_json(&serde_json::json!({
                "authenticated": true,
                "expires_at": expires_at,
            }));
        }
        None => {
            crate::output::print_json(&serde_json::json!({"authenticated": false}));
        }
    }
    Ok(())
}

pub async fn get_authenticated_client(client_id: &str) -> Result<AuthCodePkceSpotify> {
    let token = load_token()
        .ok_or_else(|| anyhow!("Not authenticated. Run `spotify-dj auth` to log in."))?;

    let creds = Credentials::new_pkce(client_id);
    let oauth = build_oauth();
    let spotify = AuthCodePkceSpotify::new(creds, oauth);

    {
        let mut guard = spotify
            .token
            .lock()
            .await
            .map_err(|_| anyhow!("Failed to acquire token lock"))?;
        *guard = Some(token);
    }

    Ok(spotify)
}
