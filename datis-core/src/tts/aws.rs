use std::str::FromStr;

use rusoto_core::request::HttpClient;
use rusoto_core::Region;
use rusoto_credential::StaticProvider;
use rusoto_polly::{Polly, PollyClient, SynthesizeSpeechInput};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceKind {
    Nicole,
    Russell,
    Amy,
    Emma,
    Brian,
    Aditi,
    Raveena,
    Ivy,
    Joanna,
    Kendra,
    Kimberly,
    Salli,
    Joey,
    Justin,
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
) -> Result<Vec<i16>, anyhow::Error> {
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
    // FIXME: use await once rusoto migrated to std futures (https://github.com/rusoto/rusoto/pull/1498)
    let response = client.synthesize_speech(req).sync()?;

    let audio_stream = response
        .audio_stream
        .ok_or_else(|| anyhow!("Polly response did not contain an audio stream"))?;
    let i16_stream = vector_i16(audio_stream);
    Ok(i16_stream)
}

fn vector_i16(byte_stream: bytes::Bytes) -> Vec<i16> {
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
    return res;
}

impl FromStr for VoiceKind {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(json!(s))
    }
}

impl ToString for VoiceKind {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}
