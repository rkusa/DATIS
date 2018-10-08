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
    name: &'a str,
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

pub fn text_to_speech(text: &str) -> Result<Vec<u8>, Error> {
    let payload = TextToSpeechRequest {
        audio_config: AudioConfig {
            audio_encoding: "OGG_OPUS",
            sample_rate_hertz: 16_000,
            speaking_rate: 0.9,
        },
        input: Input { text },
        voice: Voice {
            language_code: "en-US",
            name: "en-US-Standard-C",
        },
    };

    let key = "AIzaSyBB9rHqNGlclJTzz6bOA4hjjRmZBpdQ1Gg";
    let url = format!(
        "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
        key
    );
    let client = reqwest::Client::new();
    let mut res = client.post(&url).json(&payload).send()?;
    let data: TextToSpeechResponse = res.json()?;
    let data = base64::decode(&data.audio_content)?;
    Ok(data)
}
