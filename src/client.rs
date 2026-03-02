use anyhow::{anyhow, Result};
use rspotify::{prelude::*, AuthCodePkceSpotify};

// Values baked in at compile time (set the env vars when running `cargo install`).
// Runtime env vars take precedence so credentials can still be overridden without
// a rebuild — useful for switching between Spotify apps during development.
const BAKED_CLIENT_ID: Option<&str> = option_env!("SPOTIFY_CLIENT_ID");
const BAKED_CLIENT_SECRET: Option<&str> = option_env!("SPOTIFY_CLIENT_SECRET");

pub fn client_id() -> Option<String> {
    std::env::var("SPOTIFY_CLIENT_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| BAKED_CLIENT_ID.map(str::to_string))
}

pub fn client_secret() -> Option<String> {
    std::env::var("SPOTIFY_CLIENT_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| BAKED_CLIENT_SECRET.map(str::to_string))
}

pub async fn build_client() -> Result<AuthCodePkceSpotify> {
    let client_id = client_id().ok_or_else(|| {
        anyhow!("SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET must be set.")
    })?;

    let spotify = crate::auth::get_authenticated_client(&client_id).await?;

    let is_expired = {
        let guard = spotify
            .token
            .lock()
            .await
            .map_err(|_| anyhow::anyhow!("Failed to acquire token lock"))?;
        match *guard {
            Some(ref t) => t.is_expired(),
            None => false,
        }
    };

    if is_expired {
        spotify
            .refresh_token()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to refresh access token: {}", e))?;

        let guard = spotify
            .token
            .lock()
            .await
            .map_err(|_| anyhow::anyhow!("Failed to acquire token lock after refresh"))?;
        if let Some(ref t) = *guard {
            crate::auth::save_token(t)?;
        }
    }

    Ok(spotify)
}
