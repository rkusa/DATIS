use std::sync::{Arc, Condvar, Mutex};

use thiserror::Error;
use tokio::task;
use win_media::foundation::{AsyncOperationCompletedHandler, AsyncStatus};
use win_media::media::speech_synthesis::SpeechSynthesizer;
use win_media::windows::foundation::IAsyncOperation;
use win_media::windows::storage::streams::DataReader;

pub async fn tts(ssml: impl Into<String>, voice: Option<&str>) -> Result<Vec<u8>, Error> {
    let ssml = ssml.into();
    let voice = voice.map(String::from);
    let buf = task::spawn_blocking(move || -> Result<Vec<u8>, Error> {
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
                log::warn!("Could not find any english Windows TTS voice" );
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
        let task = synth.synthesize_ssml_to_stream_async(ssml)?;

        let stream = block_on(task)?;
        let size = stream.size()?;

        let rd = DataReader::create_data_reader(stream.get_input_stream_at(0)?)?;
        block_on(rd.load_async(size as u32)?)?;

        let mut buf = vec![0u8; size as usize];
        rd.read_bytes(buf.as_mut_slice())?;

        Ok(buf)
    }).await.unwrap()?;

    Ok(buf)
}

fn block_on<T: win_media::RuntimeType + 'static>(
    task: impl Into<IAsyncOperation<T>>,
) -> Result<T, AsyncOperationError> {
    let pair = Arc::new((Mutex::new(None), Condvar::new()));
    let pair2 = pair.clone();

    task.into()
        .set_completed(AsyncOperationCompletedHandler::new(move |op, status| {
            let (lock, cvar) = &*pair2;

            let result = match status {
                AsyncStatus::Canceled => Err(AsyncOperationError::Canceled),
                AsyncStatus::Completed => Ok(op.get_results()?),
                AsyncStatus::Error => {
                    Err(AsyncOperationError::Failed(op.error_code()?.value as u32))
                }
                _ => return Ok(()),
            };

            let mut completed = lock.lock().unwrap();
            *completed = Some(result);
            // Notify the condvar that the value has changed.
            cvar.notify_one();

            Ok(())
        }))?;

    let (lock, cvar) = &*pair;
    let mut completed = lock.lock().unwrap();
    while (*completed).is_none() {
        completed = cvar.wait(completed).unwrap();
    }

    completed.take().unwrap()
}

#[derive(Error, Debug)]
pub enum AsyncOperationError {
    #[error("The async operation got canceled")]
    Canceled,
    #[error("The async operation failed with the error code: {0}")]
    Failed(u32),
}

impl From<win_media::Error> for AsyncOperationError {
    fn from(err: win_media::Error) -> Self {
        AsyncOperationError::Failed(err.code().0)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Calling WinRT API failed with error code: {0}")]
    WinRT(u32),
    #[error("An async operation failed: {0}")]
    AsyncOperation(#[from] AsyncOperationError),
}

impl From<win_media::Error> for Error {
    fn from(err: win_media::Error) -> Self {
        Error::WinRT(err.code().0)
    }
}
