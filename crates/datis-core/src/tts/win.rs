use std::process::Command;

use audiopus::{coder::Encoder, Application, Channels, SampleRate};

#[derive(Clone)]
pub struct WindowsConfig {
    pub executable_path: Option<String>,
}

pub async fn text_to_speech(
    text: &str,
    config: &WindowsConfig,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    let path = if let Some(ref executable_path) = config.executable_path {
        executable_path.to_string() + "win-tts.exe"
    } else {
        "win-tts.exe".to_string()
    };
    debug!("Executing {} ...", path);
    // TODO: migrate to async version of Command
    let output = Command::new(path).arg(text).output()?;

    if !output.stderr.is_empty() {
        return Err(anyhow!(
            "Error calling Windows' TTS: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let audio_stream = vector_i16(output.stdout.into());

    const MONO_20MS: usize = 16000 * 1 * 20 / 1000;
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
