use crate::bindings::Windows::Media::SpeechSynthesis::SpeechSynthesizer;
use crate::bindings::Windows::Storage::Streams::DataReader;
use thiserror::Error;
use tokio::task;

pub async fn tts(ssml: impl Into<String>, voice: Option<&str>) -> Result<Vec<u8>, Error> {
    let ssml = ssml.into();
    let voice = voice.map(String::from);

    // This big block is necessary to setup a local set to be able to run !Send futures.
    let buf = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let local = tokio::task::LocalSet::new();

            // Run the local task set.
            let buf = local
                .run_until(async move {
                    task::spawn_local(async move { tts_local(ssml, voice).await })
                        .await
                        .unwrap()
                })
                .await?;

            Ok::<Vec<u8>, Error>(buf)
        })
    })
    .await
    .unwrap()?;

    Ok(buf)
}

async fn tts_local(mut ssml: String, voice: Option<String>) -> Result<Vec<u8>, Error> {
    // Note, there does not seem to be a way to explicitly set 16000kHz, 16 audio bits per
    // sample and mono channel.

    let mut voice_info = None;
    if let Some(ref voice) = voice {
        let all_voices = SpeechSynthesizer::AllVoices()?;
        let len = all_voices.Size()? as usize;
        for i in 0..len {
            let v = all_voices.GetAt(i as u32)?;
            let lang = v.Language()?.to_string();
            if !lang.starts_with("en-") {
                continue;
            }

            let name = v.DisplayName()?.to_string();
            if name.ends_with(voice) {
                voice_info = Some(v);
                break;
            }
        }
    } else {
        // default to the first english voice in the list
        let all_voices = SpeechSynthesizer::AllVoices()?;
        let len = all_voices.Size()? as usize;
        for i in 0..len {
            let v = all_voices.GetAt(i as u32)?;
            let lang = v.Language()?.to_string();
            if lang.starts_with("en-") {
                let name = v.DisplayName()?.to_string();
                log::debug!("Using WIN voice: {}", name);
                voice_info = Some(v);
                break;
            }
        }

        if voice_info.is_none() {
            log::warn!("Could not find any english Windows TTS voice");
        }
    }

    if voice_info.is_none() {
        let all_voices = SpeechSynthesizer::AllVoices()?;
        let len = all_voices.Size()? as usize;
        log::info!("Available WIN voices are (you don't have to include the `Microsoft` prefix in the name):");
        for i in 0..len {
            let v = all_voices.GetAt(i as u32)?;
            let lang = v.Language()?.to_string();
            if !lang.starts_with("en-") {
                continue;
            }

            let name = v.DisplayName()?.to_string();
            log::info!("- {} ({})", name, lang);
        }
    }

    let synth = SpeechSynthesizer::new()?;
    if let Some(info) = voice_info {
        let lang = info.Language()?.to_string();
        ssml = ssml.replacen("xml:lang=\"en\"", &format!("xml:lang=\"{}\"", lang), 1);
        synth.SetVoice(info)?;
    }

    // the DataReader is !Send, which is why we have to process it in a local set
    let stream = synth.SynthesizeSsmlToStreamAsync(ssml)?.await?;
    let size = stream.Size()?;

    let rd = DataReader::CreateDataReader(stream.GetInputStreamAt(0)?)?;
    rd.LoadAsync(size as u32)?.await?;

    let mut buf = vec![0u8; size as usize];
    rd.ReadBytes(buf.as_mut_slice())?;

    Ok(buf)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Calling WinRT API failed with error code {0}: {1}")]
    WinRT(u32, String),
    #[error("Runtime error")]
    Io(#[from] std::io::Error),
}

impl From<windows::Error> for Error {
    fn from(err: windows::Error) -> Self {
        Error::WinRT(err.code().0, err.message())
    }
}
