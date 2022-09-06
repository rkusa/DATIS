use ogg::reading::PacketReader;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;
use std::io::Cursor;
use std::str::FromStr;

#[derive(Clone)]
pub struct AzureCognitiveServicesConfig {
    pub voice: VoiceKind,
    pub key: String,
    pub region: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceKind {
    #[serde(rename = "en-US-BenjaminRUS")]
    Benjamin,
    #[serde(rename = "en-US-AriaRUS")]
    Aria,
    #[serde(rename = "en-US-ZiraRUS")]
    Zira,
    #[serde(rename = "en-US-GuyRUS")]
    Guy,
}

pub async fn text_to_speech(
    tts: &str,
    config: &AzureCognitiveServicesConfig,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    let client = reqwest::Client::new();

    //aquire token
    let token_url = format!(
        "https://{}.api.cognitive.microsoft.com/sts/v1.0/issueToken",
        config.region
    );
    let ocp_apim_key = &config.key;
    let res = client
        .post(&token_url)
        .header("Ocp-Apim-Subscription-Key", ocp_apim_key)
        .header("Content-Length", "0")
        .send()
        .await?;

    if res.status() != StatusCode::OK {
        let err = res.text().await?;
        return Err(anyhow!("Azure error: {}", err));
    }

    let token = res.text().await?;
    let api_url = format!(
        "https://{}.tts.speech.microsoft.com/cognitiveservices/v1",
        config.region
    );

    // update lang of root XML element (`<speak />`) and wrap text in an additional `<voice />` tag
    let voice = config.voice.to_string();
    let (lang, _) = voice.split_at(5);

    let tts = tts
        .strip_prefix(r#"<speak version="1.0" xml:lang="en">"#)
        .unwrap_or(tts);
    let tts = tts.strip_suffix(r#"</speak>"#).unwrap_or(tts);
    let tts = format!(
        r#"<speak version="1.0" xml:lang="{}"><voice xml:lang="{}" name="{}">{}</voice></speak>"#,
        lang, lang, voice, tts
    );

    //make request
    let res = client
        .post(&api_url)
        .bearer_auth(token)
        .header("X-Microsoft-OutputFormat", "ogg-24khz-16bit-mono-opus")
        .header("Content-Type", "application/ssml+xml")
        .header("User-Agent", "DATIS")
        .body(tts)
        .send()
        .await?;

    if res.status() != StatusCode::OK {
        let err = res.text().await?;
        return Err(anyhow!("Azure error: {}", err));
    }

    let bytes = res.bytes().await?;
    let data = Cursor::new(bytes);
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
        f.write_str(serde_json::to_string(self).unwrap().trim_matches('"'))
    }
}
