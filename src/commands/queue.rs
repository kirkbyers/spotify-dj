use anyhow::{anyhow, Result};
use rspotify::{
    model::{PlayableId, TrackId},
    prelude::*,
    AuthCodePkceSpotify,
};

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
