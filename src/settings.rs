use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub credentials: Credentials,
    pub view: ViewSettings,
    pub translation: TranslationSettings,
    pub timings: TimingSettings,
    pub update: UpdateSettings,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Credentials {
    pub token: String,
    pub uuid: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ViewSettings {
    pub timestamp: bool,
    pub label: bool,
    pub advanced: AdvancedView,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AdvancedView {
    pub enabled: bool,
    pub custom_emoji: String,
    pub custom_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TranslationSettings {
    pub enable_translation: bool,
    pub translation_language: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TimingSettings {
    pub send_time_offset: i64,
    pub enable_autooffset: bool,
    pub autooffset: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UpdateSettings {
    pub enable_autoupdate: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            credentials: Credentials::default(),
            view: ViewSettings::default(),
            translation: TranslationSettings::default(),
            timings: TimingSettings::default(),
            update: UpdateSettings::default(),
        }
    }
}
impl Default for ViewSettings {
    fn default() -> Self { Self { timestamp: true, label: true, advanced: AdvancedView::default() } }
}
impl Default for AdvancedView {
    fn default() -> Self {
        Self { enabled: false, custom_emoji: "🎶".into(), custom_status: "[{timestamp}] [{lyrics}]".into() }
    }
}
impl Default for TranslationSettings {
    fn default() -> Self { Self { enable_translation: false, translation_language: "en-US".into() } }
}
impl Default for TimingSettings {
    fn default() -> Self { Self { send_time_offset: 500, enable_autooffset: true, autooffset: 3 } }
}
impl Default for UpdateSettings {
    fn default() -> Self { Self { enable_autoupdate: false } }
}

impl Settings {
    pub async fn load(path: impl AsRef<Path>) -> Self {
        match tokio::fs::read_to_string(path.as_ref()).await {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|error| {
                tracing::warn!(%error, "settings.json is invalid; using defaults");
                Self::default()
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                tracing::warn!(%error, "could not read settings.json; using defaults");
                Self::default()
            }
        }
    }

    pub async fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_vec_pretty(self)?;
        tokio::fs::write(path.as_ref(), json).await
            .with_context(|| format!("could not save {}", path.as_ref().display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deserializes_existing_camel_case_settings() {
        let value = r#"{"timings":{"sendTimeOffset":123,"enableAutooffset":false,"autooffset":5},"view":{"advanced":{"customEmoji":"<:x:1>"}}}"#;
        let settings: Settings = serde_json::from_str(value).unwrap();
        assert_eq!(settings.timings.send_time_offset, 123);
        assert_eq!(settings.view.advanced.custom_emoji, "<:x:1>");
        assert!(settings.view.timestamp);
    }
}
