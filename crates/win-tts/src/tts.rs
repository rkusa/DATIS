use thiserror::Error;
use tokio::task;
use win_media::media::speech_synthesis::SpeechSynthesizer;
use win_media::windows::storage::streams::DataReader;

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

async fn tts_local(ssml: String, voice: Option<String>) -> Result<Vec<u8>, Error> {
    let synth = SpeechSynthesizer::new()?;

    // Note, there does not seem to be a way to explicitly set 16000kHz, 16 audio bits per
    // sample and mono channel.
    let mut voice_found = false;
    if let Some(ref voice) = voice {
        let all_voices = SpeechSynthesizer::all_voices()?;
        let len = all_voices.size()? as usize;
        for i in 0..len {
            let v = all_voices.get_at(i as u32)?;
            let lang = v.language()?.to_string();
            if !lang.starts_with("en-") {
                continue;
            }

            let name = v.display_name()?.to_string();
            if name.ends_with(voice) {
                synth.set_voice(v)?;
                voice_found = true;
                break;
            }
        }
    } else {
        // default to the first english voice in the list
        let all_voices = SpeechSynthesizer::all_voices()?;
        let len = all_voices.size()? as usize;
        for i in 0..len {
            let v = all_voices.get_at(i as u32)?;
            let lang = v.language()?.to_string();
            if lang.starts_with("en-") {
                let name = v.display_name()?.to_string();
                log::debug!("Using WIN voice: {}", name);
                synth.set_voice(v)?;
                voice_found = true;
                break;
            }
        }

        if !voice_found {
            log::warn!("Could not find any english Windows TTS voice");
        }
    }

    if !voice_found {
        let all_voices = SpeechSynthesizer::all_voices()?;
        let len = all_voices.size()? as usize;
        log::info!("Available WIN voices are (you don't have to include the `Microsoft` prefix in the name):");
        for i in 0..len {
            let v = all_voices.get_at(i as u32)?;
            let lang = v.language()?.to_string();
            if !lang.starts_with("en-") {
                continue;
            }

            let name = v.display_name()?.to_string();
            log::info!("- {}", name);
        }
    }

    // the DataReader is !Send, which is why we have to process it in a local set
    let stream = synth.synthesize_ssml_to_stream_async(ssml)?.await?;
    let size = stream.size()?;

    let rd = DataReader::create_data_reader(stream.get_input_stream_at(0)?)?;
    rd.load_async(size as u32)?.await?;

    let mut buf = vec![0u8; size as usize];
    rd.read_bytes(buf.as_mut_slice())?;

    Ok(buf)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Calling WinRT API failed with error code {0}: {1}")]
    WinRT(u32, String),
    #[error("Runtime error")]
    Io(#[from] std::io::Error),
}

impl From<win_media::Error> for Error {
    fn from(err: win_media::Error) -> Self {
        Error::WinRT(err.code().0, err.message())
    }
}
