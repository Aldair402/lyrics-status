use super::{parse_lrc, LyricsSource};
use crate::models::SongLyrics;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

pub struct LrcLib { client: reqwest::Client }
impl LrcLib { pub fn new(client: reqwest::Client) -> Self { Self { client } } }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response { synced_lyrics: Option<String> }

#[async_trait]
impl LyricsSource for LrcLib {
    fn name(&self) -> &'static str { "LrcLib" }
    async fn lyrics(&self, track: &str, artist: &str) -> Result<SongLyrics> {
        let response = self.client.get("https://lrclib.net/api/get")
            .query(&[("track_name", track), ("artist_name", artist)]).send().await?
            .error_for_status()?.json::<Response>().await.context("invalid LrcLib response")?;
        let Some(raw) = response.synced_lyrics.filter(|s| !s.trim().is_empty()) else { bail!("no synchronized lyrics") };
        let lyrics = parse_lrc(&raw);
        if lyrics.lines.is_empty() { bail!("empty synchronized lyrics") }
        Ok(lyrics)
    }
}
