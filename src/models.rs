use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct LyricsLine {
    pub time: u64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "textTranslated")]
    pub text_translated: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SongLyrics {
    pub lines: Vec<LyricsLine>,
}

#[derive(Clone, Debug, Default)]
pub struct PlaybackState {
    pub song_name: String,
    pub song_author: String,
    pub song_id: String,
    pub song_duration: u64,
    pub song_progress: u64,
    pub lyrics: Option<SongLyrics>,
    pub current_line: Option<LyricsLine>,
    pub is_playing: bool,
}

impl PlaybackState {
    pub fn ended(&self) -> bool {
        self.song_duration > 0 && self.song_progress > self.song_duration
    }
}
