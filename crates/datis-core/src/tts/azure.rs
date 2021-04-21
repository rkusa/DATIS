use std::str::FromStr;
use std::fmt::Display;
use reqwest::StatusCode;
use std::ops::{Bound, RangeBounds};
use serde::{Deserialize, Serialize};
use serde_json::{json};
use std::io::Cursor;
use ogg::reading::PacketReader;

#[derive(Clone)]
pub struct AzureCognitiveServicesConfig {
    pub voice: VoiceKind,
    pub key: String,
    pub region: String,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceKind {
    #[serde(rename = "en-US-BenjaminRUS")]
    BenjaminRUS,
    #[serde(rename = "en-US-AriaRUS")]
    AriaRUS,
    #[serde(rename = "en-US-ZiraRUS")]
    ZiraRUS,
    #[serde(rename = "en-US-GuyRUS")]
    GuyRUS,
}

pub async fn text_to_speech( 
    tts: &str,
    config: &AzureCognitiveServicesConfig,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    
    let client = reqwest::Client::new();

    //aquire token
    let token_url = format!("https://{}.api.cognitive.microsoft.com/sts/v1.0/issueToken", config.region );
    let ocp_apim_key = &config.key;
    let res = client
                .post(&token_url)
                .header("Ocp-Apim-Subscription-Key", ocp_apim_key )
                .header("Content-Length", "0" )
                .send()
                .await?;

    if res.status() != StatusCode::OK {
        let err = res.text().await?;
        return Err(anyhow!("Azure error: {}", err));
    }
            
    let token = res.text().await?;
    let api_url = format!(
          "https://{}.tts.speech.microsoft.com/cognitiveservices/v1", config.region
    );

    //reformat xml
    let azure_voice = config.voice.to_string();    
    let language = azure_voice.substring(1, 5);
    let mut len = azure_voice.len();
    len = len - 2;
    let azure_voice2 = azure_voice.substring(1, len).to_string();
    let indexof = tts.find(">").unwrap();
    let mut speak = tts.substring(0, indexof + 1).to_string();
    len = tts.len() - (8 + speak.len());
    let content = tts.substring( indexof +1, len);
    let voice = format!("<voice xml:lang='{}' name='{}'>", language.to_string(),azure_voice2);
    speak = format!("{}{}{}{}",speak, voice, content, "</voice></speak>");

    //make request
    let res = client
                .post(&api_url)
                .bearer_auth(token)
                .header("X-Microsoft-OutputFormat","ogg-24khz-16bit-mono-opus")
                .header("Content-Type", "application/ssml+xml")
                .header("User-Agent", "DATIS")
                .body(speak)
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
      f.write_str(&serde_json::to_string(self).unwrap())
  }
}

//helper for substring, etc
trait StringUtils {
    fn substring(&self, start: usize, len: usize) -> &str;
    fn slice(&self, range: impl RangeBounds<usize>) -> &str;
}

impl StringUtils for str {
    fn substring(&self, start: usize, len: usize) -> &str {
        let mut char_pos = 0;
        let mut byte_start = 0;
        let mut it = self.chars();
        loop {
            if char_pos == start { break; }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_start += c.len_utf8();
            }
            else { break; }
        }
        char_pos = 0;
        let mut byte_end = byte_start;
        loop {
            if char_pos == len { break; }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_end += c.len_utf8();
            }
            else { break; }
        }
        &self[byte_start..byte_end]
    }
    fn slice(&self, range: impl RangeBounds<usize>) -> &str {
        let start = match range.start_bound() {
            Bound::Included(bound) | Bound::Excluded(bound) => *bound,
            Bound::Unbounded => 0,
        };
        let len = match range.end_bound() {
            Bound::Included(bound) => *bound + 1,
            Bound::Excluded(bound) => *bound,
            Bound::Unbounded => self.len(),
        } - start;
        self.substring(start, len)
    }
}