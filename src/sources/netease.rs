use super::{parse_lrc, LyricsSource};
use crate::models::SongLyrics;
use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::Deserialize;

pub struct NetEase { client: reqwest::Client }
impl NetEase { pub fn new(client: reqwest::Client) -> Self { Self { client } } }
#[derive(Deserialize)] struct Search { result: SearchResult }
#[derive(Deserialize)] #[serde(rename_all = "camelCase")] struct SearchResult { songs: Option<Vec<Song>>, song_count: u64 }
#[derive(Deserialize)] struct Song { id: u64 }
#[derive(Deserialize)] struct LyricsResponse { lrc: Option<Lrc> }
#[derive(Deserialize)] struct Lrc { lyric: String }

impl NetEase {
    async fn request(&self, url: &str) -> Result<reqwest::Response> {
        Ok(self.client.post(url).header("Referer", "https://music.163.com")
            .header("Cookie", "appver=2.0.2").header("X-Real-IP", "202.96.0.0")
            .send().await?.error_for_status()?)
    }
}
#[async_trait]
impl LyricsSource for NetEase {
    fn name(&self) -> &'static str { "NetEase Music" }
    async fn lyrics(&self, track: &str, artist: &str) -> Result<SongLyrics> {
        let query = format!("{track}-{artist}");
        let search = self.client.post("https://music.163.com/api/search/get")
            .query(&[("s", query.as_str()), ("type", "1"), ("offset", "0"), ("sub", "false"), ("limit", "5")])
            .header("Referer", "https://music.163.com").header("Cookie", "appver=2.0.2")
            .send().await?.error_for_status()?.json::<Search>().await?;
        if search.result.song_count == 0 { bail!("song not found") }
        let id = search.result.songs.and_then(|s| s.first().map(|x| x.id)).ok_or_else(|| anyhow::anyhow!("song not found"))?;
        let response = self.request(&format!("https://music.163.com/api/song/lyric?tv=-1&kv=-1&lv=-1&os=pc&id={id}")).await?
            .json::<LyricsResponse>().await?;
        let raw = response.lrc.map(|l| l.lyric).filter(|s| !s.is_empty()).ok_or_else(|| anyhow::anyhow!("lyrics not found"))?;
        let lyrics = parse_lrc(&raw);
        if lyrics.lines.is_empty() { bail!("empty synchronized lyrics") }
        Ok(lyrics)
    }
}
