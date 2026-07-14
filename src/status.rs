use crate::{models::{LyricsLine, PlaybackState}, settings::Settings, translation::Translator};
use chrono::{Duration, SecondsFormat, Utc};
use regex::Regex;
use serde_json::json;
use std::{collections::{HashSet, VecDeque}, sync::Arc, time::Instant};
use tokio::sync::RwLock;

const DISCORD_STATUS_API: &str = "https://discord.com/api/v8/users/@me/settings";
const MAX_STATUS_LENGTH: usize = 128;

#[derive(Default)]
struct AutoOffset { samples: VecDeque<u64> }
impl AutoOffset {
    fn set_limit(&mut self, limit: usize) {
        let limit = limit.max(1);
        while self.samples.len() > limit { self.samples.pop_back(); }
        while self.samples.len() < limit { self.samples.push_back(0); }
    }
    fn add(&mut self, value: u64) { self.samples.pop_back(); self.samples.push_front(value); }
    fn average(&self) -> u64 {
        if self.samples.is_empty() { 0 } else { self.samples.iter().sum::<u64>() / self.samples.len() as u64 }
    }
}

pub fn format_seconds(total: u64) -> String { format!("{}:{:02}", total / 60, total % 60) }
fn truncate(value: String) -> String { value.chars().take(MAX_STATUS_LENGTH).collect() }
fn letters_only(value: &str) -> String { value.chars().filter(|c| !matches!(c, '\'' | '"' | ',' | '.')).collect() }
fn cropped(value: &str) -> String {
    Regex::new(r"(?i)( ?- ?.+)|(\(.+\))").unwrap().replace_all(value, "").into_owned()
}

pub fn build_status(state: &PlaybackState, line: &LyricsLine, settings: &Settings) -> String {
    let timestamp = format_seconds((line.time as f64 / 1000.0).round() as u64);
    if !settings.view.advanced.enabled {
        let mut parts = Vec::new();
        if settings.view.timestamp { parts.push(format!("[{timestamp}]")); }
        if settings.view.label { parts.push("Song lyrics -".into()); }
        parts.push(line.text.replacen('♪', "🎶", 1));
        return truncate(parts.join(" "));
    }
    let mut result = settings.view.advanced.custom_status.clone();
    let replacements = [
        ("{lyrics}", line.text.clone()),
        ("{lyrics_upper}", line.text.to_uppercase()),
        ("{lyrics_lower}", line.text.to_lowercase()),
        ("{lyrics_letters_only}", letters_only(&line.text)),
        ("{lyrics_upper_letters_only}", letters_only(&line.text.to_uppercase())),
        ("{lyrics_lower_letters_only}", letters_only(&line.text.to_lowercase())),
        ("{timestamp}", timestamp),
        ("{song_name}", state.song_name.clone()),
        ("{song_name_upper}", state.song_name.to_uppercase()),
        ("{song_name_lower}", state.song_name.to_lowercase()),
        ("{song_name_cropped}", cropped(&state.song_name)),
        ("{song_name_upper_cropped}", cropped(&state.song_name.to_uppercase())),
        ("{song_name_lower_cropped}", cropped(&state.song_name.to_lowercase())),
        ("{song_author}", state.song_author.clone()),
        ("{song_author_upper}", state.song_author.to_uppercase()),
        ("{song_author_lower}", state.song_author.to_lowercase()),
    ];
    for (placeholder, value) in replacements { result = result.replace(placeholder, &value); }
    truncate(result.replacen('♪', "🎶", 1))
}

fn parse_emoji(emoji: &str) -> (Option<String>, Option<String>) {
    if emoji.is_empty() { return (None, None); }
    let custom = Regex::new(r"^<a?:([^:]+):(\d+)>$").unwrap();
    if let Some(found) = custom.captures(emoji) {
        return (
            Some(found.get(2).expect("emoji id capture").as_str().to_owned()),
            Some(found.get(1).expect("emoji name capture").as_str().to_owned()),
        );
    }
    (None, Some(emoji.into()))
}

async fn send_status(client: &reqwest::Client, token: &str, text: &str, emoji: &str) -> anyhow::Result<u64> {
    let started = Instant::now();
    let (emoji_id, emoji_name) = parse_emoji(emoji);
    let expires_at = (Utc::now() + Duration::seconds(60)).to_rfc3339_opts(SecondsFormat::Millis, true);
    client.patch(DISCORD_STATUS_API).header("Authorization", token)
        .json(&json!({ "custom_status": { "text": text, "emoji_id": emoji_id, "emoji_name": emoji_name, "expires_at": expires_at } }))
        .send().await?.error_for_status()?;
    Ok(started.elapsed().as_millis() as u64)
}

pub async fn run(
    client: reqwest::Client,
    settings: Arc<RwLock<Settings>>,
    playback: Arc<RwLock<PlaybackState>>,
    translator: Translator,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(16));
    let mut last_tick = Instant::now();
    let mut last_song = String::new();
    let mut sent = HashSet::<(String, u64)>::new();
    let mut autooffset = AutoOffset::default();
    loop {
        interval.tick().await;
        let elapsed = last_tick.elapsed().as_millis() as u64;
        last_tick = Instant::now();
        let config = settings.read().await.clone();
        autooffset.set_limit(config.timings.autooffset);
        let snapshot = {
            let mut state = playback.write().await;
            if state.is_playing { state.song_progress = state.song_progress.saturating_add(elapsed); }
            state.clone()
        };
        if snapshot.song_id != last_song { last_song = snapshot.song_id.clone(); sent.clear(); }
        if !snapshot.is_playing || snapshot.ended() || config.credentials.token.is_empty() { continue; }
        let Some(lyrics) = &snapshot.lyrics else { continue };
        let offset = if config.timings.enable_autooffset { autooffset.average() as i64 + 100 } else { config.timings.send_time_offset };
        let threshold = snapshot.song_progress.saturating_add_signed(offset);
        let Some(line) = lyrics.lines.iter().rev().find(|line| line.time < threshold && !line.text.is_empty()).cloned() else { continue };
        let key = (snapshot.song_id.clone(), line.time);
        if sent.contains(&key) { continue; }
        let mut text = build_status(&snapshot, &line, &config);
        if config.translation.enable_translation {
            text = translator.translate(&text, &config.translation.translation_language).await;
            text = truncate(text);
        }
        match send_status(&client, &config.credentials.token, &text, &config.view.advanced.custom_emoji).await {
            Ok(latency) => {
                autooffset.add(latency);
                sent.insert(key);
                playback.write().await.current_line = Some(line);
            }
            Err(error) => tracing::warn!(%error, "could not update Discord status"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn parses_unicode_and_custom_emoji() {
        assert_eq!(parse_emoji("🎶"), (None, Some("🎶".into())));
        assert_eq!(parse_emoji("<a:dance:123>"), (Some("123".into()), Some("dance".into())));
    }
    #[test] fn builds_advanced_status_and_limits_characters() {
        let state = PlaybackState { song_name: "Title (Remix)".into(), song_author: "Artist".into(), ..Default::default() };
        let line = LyricsLine { time: 61_000, text: "Hello".into(), text_translated: None };
        let mut settings = Settings::default();
        settings.view.advanced.enabled = true;
        settings.view.advanced.custom_status = "{timestamp} {song_name_cropped}: {lyrics_upper}".into();
        assert_eq!(build_status(&state, &line, &settings), "1:01 Title : HELLO");
    }
}
