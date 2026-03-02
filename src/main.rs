mod auth;
mod client;
mod commands;
mod output;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "spotify-dj", version, about = "Spotify DJ CLI for Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Spotify via OAuth 2.0 PKCE
    Auth,

    /// Show current authentication status
    #[command(name = "auth-status")]
    AuthStatus,

    /// Show currently playing track
    Now,

    /// Search Spotify for tracks, playlists, albums, or artists
    Search {
        /// Search query
        query: String,

        /// Type of search: track (default), playlist, album, artist
        #[arg(long, default_value = "track")]
        r#type: String,

        /// Number of results to return (default: 5, max: 50)
        #[arg(long, default_value = "5")]
        limit: u32,
    },

    /// Play a Spotify URI, or resume playback if no URI given
    Play {
        /// Spotify URI (track, album, or playlist) to play immediately
        uri: Option<String>,
    },

    /// Pause playback
    Pause,

    /// Skip to the next track
    Skip,

    /// Add one or more tracks to the playback queue
    #[command(name = "queue-add")]
    QueueAdd {
        /// Spotify track URIs to enqueue
        #[arg(required = true)]
        uris: Vec<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        output::print_error(&e.to_string());
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Commands that don't touch the Spotify API skip the env-var check.
    let needs_api = !matches!(cli.command, Commands::Auth | Commands::AuthStatus);

    if needs_api {
        check_env_vars()?;
    } else {
        // auth and auth-status still need SPOTIFY_CLIENT_ID to build the auth URL.
        if client::client_id().is_none() {
            return Err(anyhow!(
                "SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET must be set."
            ));
        }
    }

    match cli.command {
        Commands::Auth => {
            let client_id = client::client_id().unwrap_or_default();
            auth::run_auth_flow(&client_id).await?;
        }

        Commands::AuthStatus => {
            auth::auth_status().await?;
        }

        Commands::Now => {
            let client = client::build_client().await?;
            commands::playback::now(&client).await?;
        }

        Commands::Search {
            query,
            r#type,
            limit,
        } => {
            let client = client::build_client().await?;
            commands::search::search(&client, &query, &r#type, limit).await?;
        }

        Commands::Play { uri } => {
            let client = client::build_client().await?;
            commands::playback::play(&client, uri).await?;
        }

        Commands::Pause => {
            let client = client::build_client().await?;
            commands::playback::pause(&client).await?;
        }

        Commands::Skip => {
            let client = client::build_client().await?;
            commands::playback::skip(&client).await?;
        }

        Commands::QueueAdd { uris } => {
            let client = client::build_client().await?;
            commands::queue::queue_add(&client, &uris).await?;
        }
    }

    Ok(())
}

fn check_env_vars() -> Result<()> {
    if client::client_id().is_none() || client::client_secret().is_none() {
        return Err(anyhow!(
            "SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET must be set."
        ));
    }
    Ok(())
}
