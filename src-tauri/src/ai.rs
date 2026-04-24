use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::InputSource;
use async_openai::types::audio::AudioInput;
use async_openai::types::audio::AudioResponseFormat;
use async_openai::types::audio::CreateTranscriptionRequestArgs;

#[derive(Clone)]
pub struct AiModel {
    client: async_openai::Client<OpenAIConfig>,
    model: String,
}

impl AiModel {
    pub fn new() -> Self {
        // let config = OpenAIConfig::new().with_api_base("https://lulu.gooseread.com/v1");
        // let client = Client::with_config(config);
        // Self {
        //     client,
        //     model: "CohereLabs/cohere-transcribe-03-2026".to_string(),
        // }
        let config = OpenAIConfig::new().with_api_base("https://henry.gooseread.com/v1");
        let client = Client::with_config(config);
        Self {
            client,
            model: "Qwen/Qwen3-ASR-1.7B".to_string(),
        }
    }

    pub async fn transcribe_mp3(&self, mp3_bytes: Vec<u8>) -> anyhow::Result<String> {
        let mut request = CreateTranscriptionRequestArgs::default();
        request.file(AudioInput {
            source: InputSource::Bytes {
                filename: "audio.mp3".to_string(),
                bytes: mp3_bytes.into(),
            },
        });
        request.model(self.model.clone());
        request.response_format(AudioResponseFormat::Json);
        let resp = self
            .client
            .audio()
            .transcription()
            .create(request.build()?)
            .await?;
        Ok(prune_transcript(&resp.text).to_string())
    }
}

fn prune_transcript(transcript: &str) -> &str {
    if let Some(pos) = transcript.find("<asr_text>") {
        transcript[pos + "<asr_text>".len()..].trim()
    } else {
        transcript.trim()
    }
}
