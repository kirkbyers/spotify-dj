use anyhow::{anyhow, Result};
use rspotify::{prelude::*, AuthCodePkceSpotify};

pub async fn search(
    client: &AuthCodePkceSpotify,
    query: &str,
    search_type: &str,
    limit: u32,
) -> Result<()> {
    // Use raw HTTP to avoid rspotify deserialization quirks (e.g. FullTrack
    // requiring `external_ids` which Spotify omits on some results).
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

    let limit = limit.min(50);
    let type_param = match search_type {
        "playlist" | "album" | "artist" => search_type,
        _ => "track",
    };

    let resp = reqwest::Client::new()
        .get("https://api.spotify.com/v1/search")
        .header("Authorization", format!("Bearer {}", access_token))
        .query(&[
            ("q", query),
            ("type", type_param),
            ("limit", &limit.to_string()),
        ])
        .send()
        .await
        .map_err(|e| anyhow!("Search request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Spotify API error {}: {}", status, body));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse search response: {}", e))?;

    match type_param {
        "playlist" => {
            let items = data["playlists"]["items"]
                .as_array()
                .ok_or_else(|| anyhow!("Unexpected response shape for playlists"))?;
            let playlists: Vec<_> = items
                .iter()
                .filter(|p| !p.is_null())
                .map(|p| {
                    let owner = p["owner"]["display_name"]
                        .as_str()
                        .or_else(|| p["owner"]["id"].as_str())
                        .unwrap_or("");
                    serde_json::json!({
                        "name": p["name"].as_str().unwrap_or(""),
                        "owner": owner,
                        "uri": p["uri"].as_str().unwrap_or(""),
                        "track_count": p["tracks"]["total"].as_u64().unwrap_or(0),
                    })
                })
                .collect();
            crate::output::print_json(&serde_json::json!({"playlists": playlists}));
        }
        "album" => {
            let items = data["albums"]["items"]
                .as_array()
                .ok_or_else(|| anyhow!("Unexpected response shape for albums"))?;
            let albums: Vec<_> = items
                .iter()
                .filter(|a| !a.is_null())
                .map(|a| {
                    let artist = a["artists"][0]["name"].as_str().unwrap_or("");
                    serde_json::json!({
                        "name": a["name"].as_str().unwrap_or(""),
                        "artist": artist,
                        "uri": a["uri"].as_str().unwrap_or(""),
                    })
                })
                .collect();
            crate::output::print_json(&serde_json::json!({"albums": albums}));
        }
        "artist" => {
            let items = data["artists"]["items"]
                .as_array()
                .ok_or_else(|| anyhow!("Unexpected response shape for artists"))?;
            let artists: Vec<_> = items
                .iter()
                .filter(|a| !a.is_null())
                .map(|a| {
                    serde_json::json!({
                        "name": a["name"].as_str().unwrap_or(""),
                        "uri": a["uri"].as_str().unwrap_or(""),
                        "popularity": a["popularity"].as_u64().unwrap_or(0),
                    })
                })
                .collect();
            crate::output::print_json(&serde_json::json!({"artists": artists}));
        }
        _ => {
            // default: track
            let items = data["tracks"]["items"]
                .as_array()
                .ok_or_else(|| anyhow!("Unexpected response shape for tracks"))?;
            let tracks: Vec<_> = items
                .iter()
                .filter(|t| !t.is_null())
                .map(|t| {
                    let artist = t["artists"][0]["name"].as_str().unwrap_or("");
                    serde_json::json!({
                        "name": t["name"].as_str().unwrap_or(""),
                        "artist": artist,
                        "album": t["album"]["name"].as_str().unwrap_or(""),
                        "uri": t["uri"].as_str().unwrap_or(""),
                        "popularity": t["popularity"].as_u64().unwrap_or(0),
                    })
                })
                .collect();
            crate::output::print_json(&serde_json::json!({"tracks": tracks}));
        }
    }

    Ok(())
}
