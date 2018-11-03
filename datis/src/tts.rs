use crate::error::Error;

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
    text: &'a str,
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

pub fn text_to_speech(gcloud_key: &str, text: &str, voice: VoiceKind) -> Result<Vec<u8>, Error> {
    let payload = TextToSpeechRequest {
        audio_config: AudioConfig {
            audio_encoding: "OGG_OPUS",
            sample_rate_hertz: 16_000,
            speaking_rate: 0.9,
        },
        input: Input { text },
        voice: Voice {
            language_code: "en-US",
            name: voice,
        },
    };

    let url = format!(
        "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
        gcloud_key
    );
    let client = reqwest::Client::new();
    let mut res = client.post(&url).json(&payload).send()?;
    let data: TextToSpeechResponse = res.json()?;
    let data = base64::decode(&data.audio_content)?;
    Ok(data)
}
