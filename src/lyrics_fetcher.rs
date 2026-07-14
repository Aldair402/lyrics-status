use crate::{models::SongLyrics, sources::LyricsSource};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::{Path, PathBuf}, sync::Arc};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheEntry {
    app_name: String,
    lines: Vec<crate::models::LyricsLine>,
}

pub struct LyricsFetcher {
    sources: Vec<Arc<dyn LyricsSource>>,
    cache_dir: PathBuf,
}

impl LyricsFetcher {
    pub fn new(cache_dir: impl Into<PathBuf>, sources: Vec<Arc<dyn LyricsSource>>) -> Self {
        Self { cache_dir: cache_dir.into(), sources }
    }

    fn cache_path(&self, track: &str, artist: &str) -> PathBuf {
        let digest = Sha256::digest(format!("{track}\0{artist}").as_bytes());
        self.cache_dir.join(format!("{digest:x}.json"))
    }

    async fn read_cache(&self, path: &Path) -> Option<(SongLyrics, String)> {
        let raw = tokio::fs::read(path).await.ok()?;
        let entry: CacheEntry = serde_json::from_slice(&raw).ok()?;
        if entry.lines.is_empty() { return None; }
        Some((SongLyrics { lines: entry.lines }, format!("Cache ({})", entry.app_name)))
    }

    async fn write_cache(&self, path: &Path, lyrics: &SongLyrics, app_name: &str) -> Result<()> {
        tokio::fs::create_dir_all(&self.cache_dir).await?;
        let entry = CacheEntry { app_name: app_name.into(), lines: lyrics.lines.clone() };
        tokio::fs::write(path, serde_json::to_vec(&entry)?).await.context("could not write lyrics cache")
    }

    pub async fn fetch(&self, track: &str, artist: &str) -> Option<(SongLyrics, String)> {
        let path = self.cache_path(track, artist);
        if let Some(cached) = self.read_cache(&path).await { return Some(cached); }
        for source in &self.sources {
            match source.lyrics(track, artist).await {
                Ok(lyrics) if !lyrics.lines.is_empty() => {
                    if let Err(error) = self.write_cache(&path, &lyrics, source.name()).await {
                        tracing::warn!(%error, "could not cache lyrics");
                    }
                    return Some((lyrics, source.name().into()));
                }
                Ok(_) => tracing::debug!(source = source.name(), "source returned no lines"),
                Err(error) => tracing::debug!(source = source.name(), %error, "lyrics source failed"),
            }
        }
        None
    }
}
