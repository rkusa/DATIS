use std::io::Cursor;
use std::str::FromStr;

use ogg::reading::PacketReader;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct GoogleCloudConfig {
    pub voice: VoiceKind,
    pub key: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AudioConfig<'a> {
    audio_encoding: &'a str,
    sample_rate_hertz: u32,
    speaking_rate: f32,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Input<'a> {
    ssml: &'a str,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Voice<'a> {
    language_code: &'a str,
    name: VoiceKind,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TextToSpeechRequest<'a> {
    audio_config: AudioConfig<'a>,
    input: Input<'a>,
    voice: Voice<'a>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TextToSpeechResponse {
    audio_content: String,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceKind {
    #[serde(rename = "en-US-Standard-B")]
    StandardB,
    #[serde(rename = "en-US-Standard-C")]
    StandardC,
    #[serde(rename = "en-US-Standard-D")]
    StandardD,
    #[serde(rename = "en-US-Standard-E")]
    StandardE,
    #[serde(rename = "en-US-Wavenet-A")]
    WavenetA,
    #[serde(rename = "en-US-Wavenet-B")]
    WavenetB,
    #[serde(rename = "en-US-Wavenet-C")]
    WavenetC,
    #[serde(rename = "en-US-Wavenet-D")]
    WavenetD,
    #[serde(rename = "en-US-Wavenet-E")]
    WavenetE,
    #[serde(rename = "en-US-Wavenet-F")]
    WavenetF,
}

pub async fn text_to_speech(
    text: &str,
    config: &GoogleCloudConfig,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    let payload = TextToSpeechRequest {
        audio_config: AudioConfig {
            audio_encoding: "OGG_OPUS",
            sample_rate_hertz: 16_000,
            speaking_rate: 0.9,
        },
        input: Input { ssml: text },
        voice: Voice {
            language_code: "en-US",
            name: config.voice,
        },
    };

    let url = format!(
        "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
        config.key
    );
    let client = reqwest::Client::new();
    let res = client.post(&url).json(&payload).send().await?;
    if res.status() != StatusCode::OK {
        let err: Value = res.json().await?;
        return Err(anyhow!("Gcloud TTL error: {}", err));
    }

    let data: TextToSpeechResponse = res.json().await?;
    let data = base64::decode(&data.audio_content)?;
    let data = Cursor::new(data);

    let mut frames = Vec::new();

    let mut audio = PacketReader::new(data);
    while let Some(pck) = audio.read_packet()? {
        frames.push(pck.data.to_vec())
    }

    Ok(frames)
}

impl FromStr for VoiceKind {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(json!(s))
    }
}
