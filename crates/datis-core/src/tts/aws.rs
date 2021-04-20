use std::fmt::Display;
use std::str::FromStr;

use audiopus::{coder::Encoder, Application, Channels, SampleRate};
use rusoto_core::request::HttpClient;
use rusoto_core::Region;
use rusoto_credential::StaticProvider;
use rusoto_polly::{Polly, PollyClient, SynthesizeSpeechInput};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceKind {
    // en-AU
    Nicole,
    Olivia,
    Russell,
    // en-GB
    Amy,
    Emma,
    Brian,
    // en-IN
    Aditi,
    Raveena,
    // en-US
    Ivy,
    Joanna,
    Kendra,
    Kimberly,
    Salli,
    Joey,
    Justin,
    Kevin,
    Matthew,
    Geraint,
}

#[derive(Clone)]
pub struct AmazonWebServicesConfig {
    pub voice: VoiceKind,
    pub key: String,
    pub secret: String,
    pub region: Region,
}

pub async fn text_to_speech(
    tts: &str,
    config: &AmazonWebServicesConfig,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    let dispatcher = HttpClient::new()?;
    let creds = StaticProvider::new(config.key.clone(), config.secret.clone(), None, None);

    //Build text_to_speech request
    let req = SynthesizeSpeechInput {
        engine: None, // TODO: allow usage of neural engine (only available for certain voices and regions!)
        language_code: None,
        lexicon_names: None,
        output_format: "pcm".to_string(),
        sample_rate: None, // defaults to 16,000
        speech_mark_types: None,
        text: tts.to_string(),
        text_type: Some("ssml".to_string()),
        voice_id: config.voice.to_string(),
    };

    let client = PollyClient::new_with(dispatcher, creds, config.region.clone());
    let response = client.synthesize_speech(req).await?;

    let audio_stream = response
        .audio_stream
        .ok_or_else(|| anyhow!("Polly response did not contain an audio stream"))?;
    let audio_stream = vector_i16(&audio_stream);

    const MONO_20MS: usize = 16000 /* * 1 channel */ * 20 / 1000;
    let enc = Encoder::new(SampleRate::Hz16000, Channels::Mono, Application::Voip)?;
    let mut pos = 0;
    let mut output = [0; 256];
    let mut frames = Vec::new();

    while pos + MONO_20MS < audio_stream.len() {
        let len = enc.encode(&audio_stream[pos..(pos + MONO_20MS)], &mut output)?;
        frames.push(output[..len].to_vec());

        pos += MONO_20MS;
    }

    Ok(frames)
}

fn vector_i16(byte_stream: &[u8]) -> Vec<i16> {
    let len = byte_stream.len();
    let mut res: Vec<i16> = Vec::new();
    let mut index_pos = 0;
    //hopefully this pushed the bits 8 bits at a time into a stream of u8
    while index_pos < len {
        let this_byte = byte_stream[index_pos];
        let next_byte = byte_stream[index_pos + 1];
        let these_converted = i16::from_le_bytes([this_byte, next_byte]);
        res.push(these_converted);
        index_pos += 2;
    }
    res
}

impl FromStr for VoiceKind {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(json!(s))
    }
}

impl Display for VoiceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
