mod lrclib;
mod netease;
mod qqmusic;

use crate::models::SongLyrics;
use anyhow::Result;
use async_trait::async_trait;

pub use lrclib::LrcLib;
pub use netease::NetEase;
pub use qqmusic::QqMusic;

#[async_trait]
pub trait LyricsSource: Send + Sync {
    fn name(&self) -> &'static str;
    async fn lyrics(&self, track: &str, artist: &str) -> Result<SongLyrics>;
}

pub(crate) fn parse_lrc(input: &str) -> SongLyrics {
    let timestamp = regex::Regex::new(r"\[(\d{1,3}):(\d{2})(?:[\.:](\d{1,3}))?\]").unwrap();
    let mut lines = Vec::new();
    for raw in input.lines() {
        let text = timestamp.replace_all(raw, "").trim().to_string();
        if text.is_empty() { continue; }
        for captures in timestamp.captures_iter(raw) {
            let minutes = captures[1].parse::<u64>().unwrap_or(0);
            let seconds = captures[2].parse::<u64>().unwrap_or(0);
            let fraction = captures.get(3).map(|m| m.as_str()).unwrap_or("0");
            let millis = match fraction.len() {
                1 => fraction.parse::<u64>().unwrap_or(0) * 100,
                2 => fraction.parse::<u64>().unwrap_or(0) * 10,
                _ => fraction[..fraction.len().min(3)].parse::<u64>().unwrap_or(0),
            };
            lines.push(crate::models::LyricsLine { time: (minutes * 60 + seconds) * 1000 + millis, text: text.clone(), text_translated: None });
        }
    }
    lines.sort_by_key(|line| line.time);
    SongLyrics { lines }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_multiple_timestamps_and_fractions() {
        let parsed = parse_lrc("[00:01.2][00:02.34]Hello\n[01:03.456]World");
        assert_eq!(parsed.lines.iter().map(|l| l.time).collect::<Vec<_>>(), vec![1200, 2340, 63456]);
    }
}
