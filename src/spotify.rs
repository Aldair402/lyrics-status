use crate::{lyrics_fetcher::LyricsFetcher, models::PlaybackState, settings::Settings};
use regex::Regex;
use reqwest::StatusCode;
use serde::Deserialize;
use std::{sync::Arc, time::Instant};
use tokio::sync::RwLock;

const DISCORD_CONNECTIONS: &str = "https://discord.com/api/v10/users/@me/connections";
const SPOTIFY_PLAYER: &str = "https://api.spotify.com/v1/me/player";

#[derive(Deserialize)] struct Connection { #[serde(rename = "type")] kind: String, access_token: Option<String> }
#[derive(Deserialize)] struct Player { is_playing: bool, progress_ms: u64, item: Option<Track> }
#[derive(Deserialize)] struct Track { id: String, name: String, duration_ms: u64, artists: Vec<Artist> }
#[derive(Deserialize)] struct Artist { name: String }

async fn spotify_token(client: &reqwest::Client, discord_token: &str) -> Option<String> {
    if discord_token.is_empty() { return None; }
    client.get(DISCORD_CONNECTIONS).header("Authorization", discord_token).send().await.ok()?
        .error_for_status().ok()?.json::<Vec<Connection>>().await.ok()?
        .into_iter().find(|connection| connection.kind == "spotify")?.access_token
}

pub async fn run(
    client: reqwest::Client,
    settings: Arc<RwLock<Settings>>,
    playback: Arc<RwLock<PlaybackState>>,
    source_name: Arc<RwLock<String>>,
    fetcher: LyricsFetcher,
) {
    let cleanup = Regex::new(r" \(.+\)").unwrap();
    let mut access_token = String::new();
    let mut last_lyrics_key = String::new();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        interval.tick().await;
        let discord_token = settings.read().await.credentials.token.clone();
        if access_token.is_empty() {
            let Some(token) = spotify_token(&client, &discord_token).await else {
                playback.write().await.is_playing = false;
                continue;
            };
            access_token = token;
        }

        let started = Instant::now();
        let mut response = match client.get(SPOTIFY_PLAYER).bearer_auth(&access_token).send().await {
            Ok(response) => response,
            Err(error) => { tracing::warn!(%error, "Spotify request failed"); continue; }
        };
        if response.status() == StatusCode::UNAUTHORIZED {
            access_token = spotify_token(&client, &discord_token).await.unwrap_or_default();
            if access_token.is_empty() { continue; }
            match client.get(SPOTIFY_PLAYER).bearer_auth(&access_token).send().await {
                Ok(retry) => response = retry,
                Err(_) => continue,
            }
        }
        if response.status() == StatusCode::NO_CONTENT || !response.status().is_success() {
            playback.write().await.is_playing = false;
            continue;
        }
        let Ok(data) = response.json::<Player>().await else { continue };
        let Some(track) = data.item else {
            playback.write().await.is_playing = false;
            continue;
        };
        let artist = track.artists.first().map(|a| a.name.clone()).unwrap_or_default();
        let name = cleanup.replace_all(&track.name, "").into_owned();
        let key = format!("{name}\0{artist}");
        {
            let mut state = playback.write().await;
            let changed = state.song_id != track.id;
            state.is_playing = data.is_playing;
            state.song_progress = data.progress_ms.saturating_add(started.elapsed().as_millis() as u64);
            state.song_duration = track.duration_ms;
            if changed {
                state.song_id = track.id;
                state.song_name = name.clone();
                state.song_author = artist.clone();
                state.lyrics = None;
                state.current_line = None;
            }
        }
        if key != last_lyrics_key {
            last_lyrics_key = key;
            *source_name.write().await = "Not fetched".into();
            if let Some((lyrics, source)) = fetcher.fetch(&name, &artist).await {
                let still_current = {
                    let state = playback.read().await;
                    state.song_name == name && state.song_author == artist
                };
                if still_current {
                    playback.write().await.lyrics = Some(lyrics);
                    *source_name.write().await = source;
                }
            }
        }
    }
}
