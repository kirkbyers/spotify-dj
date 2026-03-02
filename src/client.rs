use anyhow::Result;
use rspotify::{prelude::*, AuthCodePkceSpotify};

pub async fn build_client() -> Result<AuthCodePkceSpotify> {
    let client_id = std::env::var("SPOTIFY_CLIENT_ID")?;

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
