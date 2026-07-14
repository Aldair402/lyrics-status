use super::{parse_lrc, LyricsSource};
use crate::models::SongLyrics;
use anyhow::{bail, Result};
use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;

pub struct QqMusic { client: reqwest::Client }
impl QqMusic { pub fn new(client: reqwest::Client) -> Self { Self { client } } }
#[derive(Deserialize)] struct Search { count: u64, data: Data }
#[derive(Deserialize)] struct Data { song: Songs }
#[derive(Deserialize)] struct Songs { itemlist: Vec<Item> }
#[derive(Deserialize)] struct Item { mid: String }
#[derive(Deserialize)] struct LyricsResponse { lyric: String }

#[async_trait]
impl LyricsSource for QqMusic {
    fn name(&self) -> &'static str { "QQMusic" }
    async fn lyrics(&self, track: &str, artist: &str) -> Result<SongLyrics> {
        let key = format!("{track}-{artist}");
        let search = self.client.get("https://c.y.qq.com/splcloud/fcgi-bin/smartbox_new.fcg")
            .query(&[("inCharset", "utf-8"), ("outCharset", "utf-8"), ("key", key.as_str())])
            .header("Referer", "http://y.qq.com/portal/player.html").send().await?.error_for_status()?.json::<Search>().await?;
        if search.count == 0 { bail!("song not found") }
        let id = &search.data.song.itemlist.first().ok_or_else(|| anyhow::anyhow!("song not found"))?.mid;
        let response = self.client.get("https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg")
            .query(&[("g_tk", "5381"), ("format", "json"), ("inCharset", "utf-8"), ("outCharset", "utf-8"), ("songmid", id.as_str())])
            .header("Referer", "http://y.qq.com/portal/player.html").send().await?.error_for_status()?.json::<LyricsResponse>().await?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(response.lyric)?;
        let decoded = html_escape::decode_html_entities(&String::from_utf8(bytes)?).into_owned();
        let lyrics = parse_lrc(&decoded);
        if lyrics.lines.is_empty() { bail!("empty synchronized lyrics") }
        Ok(lyrics)
    }
}
