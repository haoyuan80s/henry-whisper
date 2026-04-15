use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::ChatCompletionRequestSystemMessage;
use async_openai::types::chat::ChatCompletionRequestUserMessage;
use async_openai::types::chat::CreateChatCompletionRequestArgs;
use async_openai::types::chat::CreateChatCompletionResponse;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde_json::json;

#[derive(Clone)]
pub struct AiModel {
    client: async_openai::Client<OpenAIConfig>,
    model: String,
}

#[derive(Clone, Copy)]
enum AudioFormat {
    Wav,
    Mp3,
}

impl AudioFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
        }
    }
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

    pub async fn chat(&self, system_message: &str, user_message: &str) -> anyhow::Result<String> {
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(1024u32)
            .model(self.model.clone())
            .messages([
                ChatCompletionRequestSystemMessage::from(system_message).into(),
                ChatCompletionRequestUserMessage::from(user_message).into(),
            ])
            .build()?;
        let resp = self.client.chat().create(request).await?;
        let msg = resp.choices[0]
            .clone()
            .message
            .content
            .unwrap_or_else(|| "No response".to_string());
        Ok(msg)
    }

    pub async fn transcribe_wav(&self, wav_bytes: Vec<u8>) -> anyhow::Result<String> {
        self.transcribe_audio(wav_bytes, AudioFormat::Wav).await
    }

    pub async fn transcribe_mp3(&self, mp3_bytes: Vec<u8>) -> anyhow::Result<String> {
        self.transcribe_audio(mp3_bytes, AudioFormat::Mp3).await
    }

    async fn transcribe_audio(
        &self,
        audio_bytes: Vec<u8>,
        format: AudioFormat,
    ) -> anyhow::Result<String> {
        let audio_b64 = BASE64_STANDARD.encode(&audio_bytes);
        let format = format.as_str();

        let resp: CreateChatCompletionResponse = self
            .client
            .chat()
            .create_byot(json!({
                "messages": [{
                    "model": self.model,
                    "role": "user",
                    "content": [
                    {
                        "type": "input_audio",
                        "input_audio": {
                            "data": audio_b64,
                            "format": format
                        }
                    }
                    ]
                }]
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
