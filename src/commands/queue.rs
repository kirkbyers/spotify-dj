use anyhow::{anyhow, Result};
use rspotify::{
    model::{PlayableId, TrackId},
    prelude::*,
    AuthCodePkceSpotify,
};

pub async fn queue_get(client: &AuthCodePkceSpotify) -> Result<()> {
    let access_token = {
        let guard = client.token.lock().await
            .map_err(|_| anyhow!("Failed to acquire token lock"))?;
        guard.as_ref().ok_or_else(|| anyhow!("Not authenticated"))?.access_token.clone()
    };

    let resp = reqwest::Client::new()
        .get("https://api.spotify.com/v1/me/player/queue")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| anyhow!("Failed to get queue: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow!("Spotify API error: {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().await
        .map_err(|e| anyhow!("Failed to parse queue: {}", e))?;

    // Extract only track-type items from the queue array
    let queue: Vec<serde_json::Value> = data["queue"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|item| item["type"].as_str() == Some("track"))
        .map(|item| serde_json::json!({
            "name": item["name"].as_str().unwrap_or(""),
            "artist": item["artists"][0]["name"].as_str().unwrap_or(""),
            "uri": item["uri"].as_str().unwrap_or(""),
            "duration_ms": item["duration_ms"].as_u64().unwrap_or(0),
        }))
        .collect();

    let total_duration_ms: u64 = queue.iter()
        .map(|t| t["duration_ms"].as_u64().unwrap_or(0))
        .sum();

    crate::output::print_json(&serde_json::json!({
        "queue": queue,
        "total_duration_ms": total_duration_ms,
    }));
    Ok(())
}

pub async fn queue_add(client: &AuthCodePkceSpotify, uris: &[String]) -> Result<()> {
    let mut count = 0usize;

    for uri in uris {
        if uri.starts_with("spotify:track:") {
            let id = TrackId::from_uri(uri)
                .map_err(|e| anyhow!("Invalid track URI '{}': {}", uri, e))?;
            client
                .add_item_to_queue(PlayableId::Track(id), None)
                .await
                .map_err(|e| anyhow!("Failed to add '{}' to queue: {}", uri, e))?;
            count += 1;
        } else {
            return Err(anyhow!(
                "queue-add only supports spotify:track: URIs, got: {}",
                uri
            ));
        }
    }

    crate::output::print_json(&serde_json::json!({"ok": true, "queued": count}));
    Ok(())
}
