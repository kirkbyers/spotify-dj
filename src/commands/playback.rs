use anyhow::{anyhow, Result};
use rspotify::{
    model::{AlbumId, ArtistId, PlayableId, PlayContextId, PlaylistId, TrackId},
    prelude::*,
    AuthCodePkceSpotify,
};

pub async fn now(client: &AuthCodePkceSpotify) -> Result<()> {
    // Use a raw HTTP request instead of client.current_playback() to avoid
    // rspotify's PlayableItem untagged-enum deserializer, which panics on
    // local files, ads, and other item shapes Spotify occasionally returns.
    let access_token = {
        let guard = client
            .token
            .lock()
            .await
            .map_err(|_| anyhow!("Failed to acquire token lock"))?;
        guard
            .as_ref()
            .ok_or_else(|| anyhow!("Not authenticated"))?
            .access_token
            .clone()
    };

    let resp = reqwest::Client::new()
        .get("https://api.spotify.com/v1/me/player")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| anyhow!("Failed to get playback state: {}", e))?;

    // 204 means no active playback session at all
    if resp.status() == reqwest::StatusCode::NO_CONTENT {
        crate::output::print_json(&serde_json::json!({"playing": false}));
        return Ok(());
    }

    if !resp.status().is_success() {
        return Err(anyhow!(
            "Spotify API error: {}",
            resp.status()
        ));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse playback state: {}", e))?;

    let is_playing = data["is_playing"].as_bool().unwrap_or(false);

    // item is null when an ad is playing or the session is in a weird state
    let track_info = if data["item"].is_object()
        && data["item"]["type"].as_str() == Some("track")
    {
        let item = &data["item"];
        let artist = item["artists"][0]["name"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Some(serde_json::json!({
            "name": item["name"].as_str().unwrap_or(""),
            "artist": artist,
            "album": item["album"]["name"].as_str().unwrap_or(""),
            "uri": item["uri"].as_str().unwrap_or(""),
            "duration_ms": item["duration_ms"].as_u64().unwrap_or(0),
            "progress_ms": data["progress_ms"].as_u64().unwrap_or(0),
        }))
    } else {
        None
    };

    let device_info = serde_json::json!({
        "name": data["device"]["name"].as_str().unwrap_or(""),
        "type": data["device"]["type"].as_str().unwrap_or(""),
        "volume": data["device"]["volume_percent"].as_u64().unwrap_or(0),
    });

    crate::output::print_json(&serde_json::json!({
        "playing": is_playing,
        "track": track_info,
        "device": device_info,
        "shuffle": data["shuffle_state"].as_bool().unwrap_or(false),
        "repeat": data["repeat_state"].as_str().unwrap_or("off"),
    }));

    Ok(())
}

pub async fn play(client: &AuthCodePkceSpotify, uri: Option<String>) -> Result<()> {
    match uri {
        None => {
            client
                .resume_playback(None, None)
                .await
                .map_err(|e| anyhow!("Failed to resume playback: {}", e))?;
        }
        Some(uri_str) => {
            play_uri(client, &uri_str).await?;
        }
    }
    crate::output::print_json(&serde_json::json!({"ok": true}));
    Ok(())
}

async fn play_uri(client: &AuthCodePkceSpotify, uri: &str) -> Result<()> {
    if uri.starts_with("spotify:track:") {
        let id = TrackId::from_uri(uri)
            .map_err(|e| anyhow!("Invalid track URI '{}': {}", uri, e))?;
        client
            .start_uris_playback([PlayableId::Track(id)], None, None, None)
            .await
            .map_err(|e| anyhow!("Failed to start playback: {}", e))?;
    } else if uri.starts_with("spotify:album:") {
        let id = AlbumId::from_uri(uri)
            .map_err(|e| anyhow!("Invalid album URI '{}': {}", uri, e))?;
        client
            .start_context_playback(PlayContextId::Album(id), None, None, None)
            .await
            .map_err(|e| anyhow!("Failed to start album playback: {}", e))?;
    } else if uri.starts_with("spotify:playlist:") {
        let id = PlaylistId::from_uri(uri)
            .map_err(|e| anyhow!("Invalid playlist URI '{}': {}", uri, e))?;
        client
            .start_context_playback(PlayContextId::Playlist(id), None, None, None)
            .await
            .map_err(|e| anyhow!("Failed to start playlist playback: {}", e))?;
    } else if uri.starts_with("spotify:artist:") {
        let id = ArtistId::from_uri(uri)
            .map_err(|e| anyhow!("Invalid artist URI '{}': {}", uri, e))?;
        client
            .start_context_playback(PlayContextId::Artist(id), None, None, None)
            .await
            .map_err(|e| anyhow!("Failed to start artist playback: {}", e))?;
    } else {
        return Err(anyhow!(
            "Unsupported URI type '{}'. Expected spotify:track:, spotify:album:, spotify:playlist:, or spotify:artist:",
            uri
        ));
    }

    Ok(())
}

pub async fn pause(client: &AuthCodePkceSpotify) -> Result<()> {
    client
        .pause_playback(None)
        .await
        .map_err(|e| anyhow!("Failed to pause playback: {}", e))?;
    crate::output::print_json(&serde_json::json!({"ok": true}));
    Ok(())
}

pub async fn skip(client: &AuthCodePkceSpotify) -> Result<()> {
    client
        .next_track(None)
        .await
        .map_err(|e| anyhow!("Failed to skip track: {}", e))?;
    crate::output::print_json(&serde_json::json!({"ok": true}));
    Ok(())
}
