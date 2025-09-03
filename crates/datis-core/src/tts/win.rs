use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceKind {
    // en-AU
    Catherine,
    James,
    // en-CA
    Linda,
    Richard,
    // en-GB
    George,
    Hazel,
    Susan,
    // en-IE
    Sean,
    // en-IN
    Heera,
    Ravi,
    // en-US
    David,
    Zira,
    Mark,
}

#[derive(Clone)]
pub struct WindowsConfig {
    pub voice: Option<VoiceKind>,
}

#[cfg(target_os = "windows")]
pub async fn text_to_speech(
    ssml: &str,
    config: &WindowsConfig,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    use audiopus::coder::Encoder;
    use audiopus::{Application, Channels, SampleRate};
    use tokio::task;

    let wav = win_tts::tts(ssml, config.voice.as_deref()).await?;

    let frames = task::spawn_blocking(move || {
        let audio_stream = vector_i16(wav.into());

        const MONO_20MS: usize = 16000 /* 1 channel */ * 20 / 1000;
        let enc = Encoder::new(SampleRate::Hz16000, Channels::Mono, Application::Voip)?;
        let mut pos = 0;
        let mut output = [0; 256];
        let mut frames = Vec::new();

        while pos + MONO_20MS < audio_stream.len() {
            let len = enc.encode(&audio_stream[pos..(pos + MONO_20MS)], &mut output)?;
            frames.push(output[..len].to_vec());

            pos += MONO_20MS;
        }

        Ok::<_, audiopus::Error>(frames)
    })
    .await
    .unwrap()?;

    Ok(frames)
}

#[cfg(not(target_os = "windows"))]
pub async fn text_to_speech(
    _ssml: &str,
    _config: &WindowsConfig,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    Err(anyhow!("WIN voice only supported on Windows"))
}

#[cfg(target_os = "windows")]
fn vector_i16(byte_stream: bytes::Bytes) -> Vec<i16> {
    let len = byte_stream.len();
    let mut res: Vec<i16> = Vec::new();
    let mut index_pos = 0;
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

impl Deref for VoiceKind {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            VoiceKind::Catherine => "Catherine",
            VoiceKind::James => "James",
            VoiceKind::Linda => "Linda",
            VoiceKind::Richard => "Richard",
            VoiceKind::George => "George",
            VoiceKind::Hazel => "Hazel",
            VoiceKind::Susan => "Susan",
            VoiceKind::Sean => "Sean",
            VoiceKind::Heera => "Heera",
            VoiceKind::Ravi => "Ravi",
            VoiceKind::David => "David",
            VoiceKind::Zira => "Zira",
            VoiceKind::Mark => "Mark",
        }
    }
}

impl Display for VoiceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}
