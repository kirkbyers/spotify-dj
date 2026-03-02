# spotify-dj

A Rust CLI that wraps the Spotify Web API. It's the foundational piece of the Claude Code Spotify DJ system — a Claude sub-agent that watches your dev session and adjusts the music to match what you're working on.

All output is JSON, so it's easy to parse from shell scripts or Claude hooks.

---

## Getting Started

### Prerequisites

- Rust toolchain (`rustup` + `cargo`)
- A Spotify account (free or premium, though playback control requires premium)
- A Spotify app with a Client ID and Client Secret — create one at the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)

In your Spotify app settings, add `http://127.0.0.1:8888/callback` as a Redirect URI.

### Build & Install

The credentials are baked into the binary at compile time, so you only need to set them once — when you run `cargo install`. After that the binary works anywhere without any environment variables.

```bash
SPOTIFY_CLIENT_ID="your_client_id_here" \
SPOTIFY_CLIENT_SECRET="your_client_secret_here" \
cargo install --path .
```

If you need to swap credentials later, just re-run the install command with the new values.

You can still override the baked-in credentials at runtime by setting the env vars normally — useful if you're switching between Spotify apps during development.

### Authenticate

Run this once to kick off the OAuth flow:

```bash
spotify-dj auth
```

Your browser will open to Spotify's login page. After you approve, the CLI captures the callback on `http://127.0.0.1:8888/callback` and saves a refresh token to `~/.config/spotify-dj/credentials.json`. You won't need to log in again — tokens are refreshed automatically on each invocation.

---

## Usage

### Check what's playing

```bash
spotify-dj now
```

```json
{
  "playing": true,
  "track": {
    "name": "Midnight City",
    "artist": "M83",
    "album": "Hurry Up, We're Dreaming",
    "uri": "spotify:track:6GyFP1nfCDB8lbD2bG0Hq9",
    "duration_ms": 243894,
    "progress_ms": 45231
  },
  "device": {
    "name": "MacBook Pro",
    "type": "Computer",
    "volume": 65
  },
  "shuffle": false,
  "repeat": "off"
}
```

Returns `{"playing": false}` if nothing is active.

### Search

```bash
# Tracks (default)
spotify-dj search "lo-fi focus"

# Playlists, with a custom result limit
spotify-dj search "coding vibes" --type playlist --limit 10

# Albums or artists
spotify-dj search "Tycho" --type album
spotify-dj search "Bonobo" --type artist
```

```json
{
  "tracks": [
    {
      "name": "Chill Lo-Fi Study Beats",
      "artist": "Lo-Fi Collective",
      "album": "Focus Sessions Vol. 1",
      "uri": "spotify:track:abc123",
      "popularity": 72
    }
  ]
}
```

`--limit` defaults to `5`, max `50`.

### Playback control

```bash
# Resume if paused
spotify-dj play

# Play a specific track, album, or playlist immediately
spotify-dj play spotify:track:6GyFP1nfCDB8lbD2bG0Hq9
spotify-dj play spotify:album:5GwhhH4WEoTROJoaB1rDXJ
spotify-dj play spotify:playlist:37i9dQZF1DX8NTLI2TtZa6

# Pause
spotify-dj pause

# Skip to next track
spotify-dj skip
```

All three return `{"ok": true}` on success.

### Queue management

```bash
# Add one track
spotify-dj queue-add spotify:track:6GyFP1nfCDB8lbD2bG0Hq9

# Add several at once
spotify-dj queue-add spotify:track:abc123 spotify:track:def456 spotify:track:ghi789
```

```json
{"ok": true, "queued": 3}
```

### Auth status

```bash
spotify-dj auth-status
```

```json
{"authenticated": true, "expires_at": "2025-03-01T21:00:00+00:00"}
```

---

## Error handling

Every error — bad URI, no active device, network failure, missing credentials — comes back as JSON on stdout with a non-zero exit code:

```json
{"error": "No active playback device found."}
```

This makes it straightforward to detect failures in scripts or Claude hooks without parsing unstructured text.

---

## Project layout

```
src/
├── main.rs           # CLI entry point (clap), top-level error-to-JSON handler
├── auth.rs           # OAuth PKCE flow, token load/save/refresh
├── client.rs         # Authenticated Spotify client builder
├── output.rs         # print_json / print_error helpers
└── commands/
    ├── playback.rs   # now, play, pause, skip
    ├── queue.rs      # queue-add
    └── search.rs     # search
```
