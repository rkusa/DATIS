use std::fmt::Display;
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
    #[serde(rename = "en-AU-Standard-A")]
    EnAuStandardA,
    #[serde(rename = "en-AU-Standard-B")]
    EnAuStandardB,
    #[serde(rename = "en-AU-Standard-C")]
    EnAuStandardC,
    #[serde(rename = "en-AU-Standard-D")]
    EnAuStandardD,
    #[serde(rename = "en-AU-Wavenet-A")]
    EnAuWavenetA,
    #[serde(rename = "en-AU-Wavenet-B")]
    EnAuWavenetB,
    #[serde(rename = "en-AU-Wavenet-C")]
    EnAuWavenetC,
    #[serde(rename = "en-AU-Wavenet-D")]
    EnAuWavenetD,
    #[serde(rename = "en-IN-Standard-A")]
    EnInStandardA,
    #[serde(rename = "en-IN-Standard-B")]
    EnInStandardB,
    #[serde(rename = "en-IN-Standard-C")]
    EnInStandardC,
    #[serde(rename = "en-IN-Standard-D")]
    EnInStandardD,
    #[serde(rename = "en-IN-Wavenet-A")]
    EnInWavenetA,
    #[serde(rename = "en-IN-Wavenet-B")]
    EnInWavenetB,
    #[serde(rename = "en-IN-Wavenet-C")]
    EnInWavenetC,
    #[serde(rename = "en-IN-Wavenet-D")]
    EnInWavenetD,
    #[serde(rename = "en-GB-Standard-A")]
    EnGbStandardA,
    #[serde(rename = "en-GB-Standard-B")]
    EnGbStandardB,
    #[serde(rename = "en-GB-Standard-C")]
    EnGbStandardC,
    #[serde(rename = "en-GB-Standard-D")]
    EnGbStandardD,
    #[serde(rename = "en-GB-Standard-F")]
    EnGbStandardF,
    #[serde(rename = "en-GB-Wavenet-A")]
    EnGbWavenetA,
    #[serde(rename = "en-GB-Wavenet-B")]
    EnGbWavenetB,
    #[serde(rename = "en-GB-Wavenet-C")]
    EnGbWavenetC,
    #[serde(rename = "en-GB-Wavenet-D")]
    EnGbWavenetD,
    #[serde(rename = "en-GB-Wavenet-F")]
    EnGbWavenetF,
    #[serde(rename = "en-US-Standard-B")]
    EnUsStandardB,
    #[serde(rename = "en-US-Standard-C")]
    EnUsStandardC,
    #[serde(rename = "en-US-Standard-D")]
    EnUsStandardD,
    #[serde(rename = "en-US-Standard-E")]
    EnUsStandardE,
    #[serde(rename = "en-US-Standard-G")]
    EnUsStandardG,
    #[serde(rename = "en-US-Standard-H")]
    EnUsStandardH,
    #[serde(rename = "en-US-Standard-I")]
    EnUsStandardI,
    #[serde(rename = "en-US-Standard-J")]
    EnUsStandardJ,
    #[serde(rename = "en-US-Wavenet-A")]
    EnUsWavenetA,
    #[serde(rename = "en-US-Wavenet-B")]
    EnUsWavenetB,
    #[serde(rename = "en-US-Wavenet-C")]
    EnUsWavenetC,
    #[serde(rename = "en-US-Wavenet-D")]
    EnUsWavenetD,
    #[serde(rename = "en-US-Wavenet-E")]
    EnUsWavenetE,
    #[serde(rename = "en-US-Wavenet-F")]
    EnUsWavenetF,
    #[serde(rename = "en-US-Wavenet-G")]
    EnUsWavenetG,
    #[serde(rename = "en-US-Wavenet-H")]
    EnUsWavenetH,
    #[serde(rename = "en-US-Wavenet-I")]
    EnUsWavenetI,
    #[serde(rename = "en-US-Wavenet-J")]
    EnUsWavenetJ,
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

impl Display for VoiceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).unwrap())
    }
}
