use anyhow::{Context, anyhow};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use base64::Engine as _;
use henry_whisper_shared::AppSettings;
use serde_json::{Value, json};

const TRANSCRIPTION_PROMPT: &str = concat!(
    "Transcribe the provided audio verbatim. ",
    "Return only the transcript text with no summary, no explanation, and no markdown. ",
    "If the audio is empty or contains no intelligible speech, return an empty string."
);

#[derive(Clone)]
pub struct AiModel {
    client: async_openai::Client<OpenAIConfig>,
    model: String,
}

impl AiModel {
    pub fn from_settings(settings: &AppSettings) -> Self {
        let api_base = resolve_api_base(settings);
        let api_key = resolve_api_key(settings);

        let config = OpenAIConfig::new()
            .with_api_base(api_base)
            .with_api_key(api_key);

        let client = Client::with_config(config);
        let model = settings.ai_model.trim();

        Self {
            client,
            model: if model.is_empty() {
                AppSettings::default().ai_model
            } else {
                model.to_string()
            },
        }
    }

    pub async fn transcribe_mp3(&self, mp3_bytes: Vec<u8>) -> anyhow::Result<String> {
        if self.model.is_empty() {
            return Err(anyhow!("AI model is not configured"));
        }

        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(mp3_bytes);
        let request = json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": TRANSCRIPTION_PROMPT
                    },
                    {
                        "type": "input_audio",
                        "input_audio": {
                            "data": audio_base64,
                            "format": "mp3"
                        }
                    }
                ]
            }],
            "temperature": 0
        });

        let response: Value = self.client.chat().create_byot(request).await?;
        let transcript = extract_chat_completion_text(&response)
            .context("chat completion response did not include transcript text")?;

        Ok(prune_transcript(&transcript).to_string())
    }
}

fn resolve_api_key(settings: &AppSettings) -> String {
    let configured = settings.ai_api_key.trim();
    if !configured.is_empty() {
        return configured.to_string();
    }

    std::env::var("OPENAI_API_KEY").unwrap_or_default()
}

fn resolve_api_base(settings: &AppSettings) -> String {
    let configured = settings.ai_api_base.trim();
    if !configured.is_empty() {
        return configured.to_string();
    }

    std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| AppSettings::default().ai_api_base)
}

fn extract_chat_completion_text(response: &Value) -> Option<String> {
    let content = response
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?;

    match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(extract_content_part_text)
                .collect::<Vec<_>>()
                .join("\n");
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
        _ => None,
    }
}

fn extract_content_part_text(part: &Value) -> Option<&str> {
    part.get("text").and_then(Value::as_str).or_else(|| {
        part.get("type")
            .and_then(Value::as_str)
            .filter(|ty| *ty == "text")
            .and_then(|_| part.get("content"))
            .and_then(Value::as_str)
    })
}

fn prune_transcript(transcript: &str) -> &str {
    if let Some(pos) = transcript.find("<asr_text>") {
        transcript[pos + "<asr_text>".len()..].trim()
    } else {
        transcript.trim()
    }
}
