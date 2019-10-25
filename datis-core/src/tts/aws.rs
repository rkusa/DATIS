use std::env;
use std::str::FromStr;

use futures::compat::Future01CompatExt;
use rusoto_core::Region;
use rusoto_credential::{EnvironmentProvider, ProvideAwsCredentials};
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
    pub region: String,
}

pub async fn text_to_speech(
    tts: &str,
    config: &AmazonWebServicesConfig,
) -> Result<Vec<i16>, anyhow::Error> {
    //credentials
    env::set_var("AWS_ACCESS_KEY_ID", &config.key);
    env::set_var("AWS_SECRET_ACCESS_KEY", &config.secret);
    //this is new line
    env::set_var("AWS_REGION", &config.region);

    //put them into a class
    let _creds = EnvironmentProvider::default()
        .credentials()
        .compat()
        .await?;

    //Add output format
    let output_format = "pcm";

    //Add text type
    let txt_type: Option<String> = Some(String::from("ssml"));

    //log request string
    debug!("Sending polly this ssml string {}.", String::from(tts));

    //Build text_to_speech request
    let _text_to_speech_request = SynthesizeSpeechInput {
        language_code: Option::default(),
        lexicon_names: Option::default(),
        output_format: String::from(output_format),
        sample_rate: Option::default(),
        speech_mark_types: Option::default(),
        text: String::from(tts),
        text_type: txt_type,
        voice_id: config.voice.to_string(),
    };

    //build client which seems to default to the credential provider
    //let _polly_client = PollyClient::new(Region::UsEast1);
    //new line to test region env variable
    let _polly_client = PollyClient::new(Region::default());

    //write log
    debug!("Sending request to Amazon.");
    //post request
    let response = _polly_client
        .synthesize_speech(_text_to_speech_request)
        .sync()?;

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
        serde_json::to_string(self).unwrap()
    }
}
