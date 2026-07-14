use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Translator {
    client: reqwest::Client,
    cache: Arc<Mutex<HashMap<String, String>>>,
}
impl Translator {
    pub fn new(client: reqwest::Client) -> Self { Self { client, cache: Arc::new(Mutex::new(HashMap::new())) } }
    pub async fn translate(&self, text: &str, language: &str) -> String {
        if text.trim().is_empty() { return text.into(); }
        let key = format!("{language}\0{text}");
        if let Some(value) = self.cache.lock().await.get(&key).cloned() { return value; }
        let result = async {
            let data = self.client.get("https://translate.googleapis.com/translate_a/single")
                .query(&[("client", "gtx"), ("sl", "auto"), ("tl", language), ("dt", "t"), ("q", text)])
                .send().await?.error_for_status()?.json::<Value>().await?;
            let parts = data.get(0).and_then(Value::as_array).ok_or_else(|| anyhow::anyhow!("unexpected translation response"))?;
            let translated = parts.iter().filter_map(|part| part.get(0)?.as_str()).collect::<String>().trim().to_string();
            Ok::<String, anyhow::Error>(translated)
        }.await;
        match result {
            Ok(translated) if !translated.is_empty() && !translated.eq_ignore_ascii_case(text.trim()) => {
                self.cache.lock().await.insert(key, translated.clone()); translated
            }
            Ok(_) => text.into(),
            Err(error) => { tracing::warn!(%error, "translation failed"); text.into() }
        }
    }
}
