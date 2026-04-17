use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::CreateChatCompletionResponse;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde_json::json;

#[derive(Clone)]
pub struct AiModel {
    client: async_openai::Client<OpenAIConfig>,
    model: String,
}

impl AiModel {
    pub fn new(base_url: &str, model: &str) -> Self {
        let config = OpenAIConfig::new().with_api_base(base_url);
        let client = Client::with_config(config);
        Self {
            client,
            model: model.to_string(),
        }
    }

    pub async fn transcribe_mp3_with_prompt(
        &self,
        system_message: &str,
        mp3_bytes: Vec<u8>,
    ) -> anyhow::Result<String> {
        self.transcribe_audio(mp3_bytes, system_message).await
    }

    async fn transcribe_audio(
        &self,
        audio_bytes: Vec<u8>,
        system_message: &str,
    ) -> anyhow::Result<String> {
        let audio_b64 = BASE64_STANDARD.encode(&audio_bytes);
        let mut messages = Vec::new();
        messages.push(json!({
            "role": "system",
            "content": system_message,
        }));

        let mut content = vec![json!({
            "type": "text",
            "text": "Process this audio and return only the final transcript.",
        })];
        content.push(json!({
            "type": "input_audio",
            "input_audio": {
                "data": audio_b64,
                "format": "mp3"
            }
        }));
        messages.push(json!({
            "role": "user",
            "content": content,
        }));

        let resp: CreateChatCompletionResponse = self
            .client
            .chat()
            .create_byot(json!({
                "model": self.model,
                "messages": messages,
            }))
            .await?;
        let msg = resp.choices[0]
            .clone()
            .message
            .content
            .unwrap_or_else(|| "No transcript".to_string());
        Ok(prune_transcript(&msg).to_string())
    }
}

fn prune_transcript(transcript: &str) -> &str {
    if let Some(pos) = transcript.find("<asr_text>") {
        transcript[pos + "<asr_text>".len()..].trim()
    } else {
        transcript.trim()
    }
}
