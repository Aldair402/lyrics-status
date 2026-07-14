mod lyrics_fetcher;
mod models;
mod server;
mod settings;
mod sources;
mod spotify;
mod status;
mod translation;

use crate::{lyrics_fetcher::LyricsFetcher, models::PlaybackState, settings::Settings, sources::{LrcLib, LyricsSource, NetEase, QqMusic}, translation::Translator};
use anyhow::Context;
use std::{
    io::{self, IsTerminal, Write},
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .without_time()
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("lyrics_status=info")),
        )
        .init();
    let client = reqwest::Client::builder().user_agent(concat!("LyricsStatus/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(15)).build().context("could not create HTTP client")?;
    let mut loaded = Settings::load("settings.json").await;
    if loaded.credentials.uuid.is_empty() {
        loaded.credentials.uuid = uuid::Uuid::new_v4().to_string();
        loaded.save("settings.json").await?;
    }
    let settings = Arc::new(RwLock::new(loaded));
    let playback = Arc::new(RwLock::new(PlaybackState::default()));
    let source_name = Arc::new(RwLock::new("Not fetched".to_string()));
    let sources: Vec<Arc<dyn LyricsSource>> = vec![
        Arc::new(LrcLib::new(client.clone())), Arc::new(NetEase::new(client.clone())), Arc::new(QqMusic::new(client.clone())),
    ];
    let fetcher = LyricsFetcher::new("cache", sources);
    tokio::spawn(spotify::run(client.clone(), settings.clone(), playback.clone(), source_name.clone(), fetcher));
    tokio::spawn(status::run(client.clone(), settings.clone(), playback.clone(), Translator::new(client.clone())));
    tokio::spawn(display(playback.clone(), source_name));
    if settings.read().await.update.enable_autoupdate { tokio::spawn(check_update(client)); }
    server::run(settings).await
}

async fn display(playback: Arc<RwLock<PlaybackState>>, source: Arc<RwLock<String>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let interactive = io::stdout().is_terminal();

    loop {
        interval.tick().await;

        let state = playback.read().await.clone();
        let source = source.read().await.clone();
        let song = value_or_placeholder(&state.song_name);
        let artist = value_or_placeholder(&state.song_author);
        let lyric = state
            .current_line
            .as_ref()
            .map(|line| value_or_placeholder(&line.text))
            .unwrap_or("—");
        let playback_status = if state.is_playing { "Reproduciendo" } else { "En espera" };
        let elapsed = status::format_seconds(state.song_progress / 1000);
        let duration = status::format_seconds(state.song_duration / 1000);

        if interactive {
            let mut stdout = io::stdout().lock();
            let _ = write!(
                stdout,
                "\x1b[2J\x1b[H\
                 \x1b[1;36mLyricsStatus\x1b[0m  ·  {playback_status}\n\
                 \x1b[90m────────────────────────────────────────────────────────\x1b[0m\n\
                 \x1b[1mCanción\x1b[0m   {song}\n\
                 \x1b[1mArtista\x1b[0m   {artist}\n\
                 \x1b[1mTiempo\x1b[0m    {elapsed} / {duration}\n\
                 \x1b[1mLetra\x1b[0m     {lyric}\n\
                 \x1b[1mFuente\x1b[0m    {source}\n\n\
                 \x1b[90mPanel: http://localhost:8999\x1b[0m\n"
            );
            let _ = stdout.flush();
        } else {
            println!("{playback_status} · {song} — {artist} · {elapsed}/{duration} · {lyric} [{source}]");
        }
    }
}

fn value_or_placeholder(value: &str) -> &str {
    if value.is_empty() { "—" } else { value }
}
async fn check_update(client: reqwest::Client) {
    let result = async {
        let remote = client.get("https://raw.githubusercontent.com/OvalQuilter/lyrics-status/v3/VERSION").send().await?.error_for_status()?.text().await?;
        let local = tokio::fs::read_to_string("VERSION").await?;
        Ok::<(String, String), anyhow::Error>((local, remote))
    }.await;
    match result {
        Ok((local, remote)) if local.trim() != remote.trim() => tracing::warn!(current = local.trim(), available = remote.trim(), "an update is available; rebuild from the latest source"),
        Err(error) => tracing::warn!(%error, "automatic update check failed"),
        _ => tracing::info!("LyricsStatus is up to date"),
    }
}
