use std::sync::{Arc, Mutex, Condvar};

use thiserror::Error;
use winrt::foundation::{AsyncOperationCompletedHandler, AsyncStatus};
use winrt::media::speech_synthesis::SpeechSynthesizer;
use winrt::windows::foundation::IAsyncOperation;
use winrt::windows::storage::streams::DataReader;
use tokio::task;

pub async fn tts(ssml: impl Into<String>, voice: Option<&str>) -> Result<Vec<u8>, Error> {
    let ssml = ssml.into();
    let voice = voice.map(String::from);
    let buf = task::spawn_blocking(move || {
        let synth = SpeechSynthesizer::new()?;

        // Note, there does not seem to be a way to explicitly set 16000kHz, 16 audio bits per
        // sample and mono channel.

        if let Some(ref voice) = voice {
            let all_voices = SpeechSynthesizer::all_voices()?;
            let len = all_voices.size()? as usize;
            let mut found = false;
            for i in 0..len {
                let v = all_voices.get_at(i as u32)?;
                let lang = v.language()?.to_string();
                if !lang.starts_with("en-") {
                    continue;
                }

                let name = v.display_name()?.to_string();
                if name.ends_with(voice) {
                    synth.set_voice(v)?;
                    found = true;
                    break;
                }
            }

            if !found {
                log::warn!(
                    "WIN voice `{}` not found, using default voice instead",
                    voice
                );

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
        }

        // the DataReader is !Send, which is why we have to process it in a local set
        let task = synth.synthesize_ssml_to_stream_async(ssml)?;

        let stream = block_on(task)?;
        let size = stream.size()?;

        let rd = DataReader::create_data_reader(stream.get_input_stream_at(0)?)?;
        block_on(rd.load_async(size as u32)?)?;

        let mut buf = vec![0u8; size as usize];
        rd.read_bytes(buf.as_mut_slice())?;

        Ok::<Vec<u8>, AsyncOperationError>(buf)
    }).await.unwrap()?;

    Ok(buf)
}

fn block_on<T: winrt::RuntimeType + 'static>(
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
                AsyncStatus::Error => Err(AsyncOperationError::Failed(op.error_code()?.value)),
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
    while (*completed).is_none()  {
        completed = cvar.wait(completed ).unwrap();
    }

    completed.take().unwrap()
}

#[derive(Error, Debug)]
pub enum AsyncOperationError {
    #[error("The async operation got canceled")]
    Canceled,
    #[error("The async operation failed with the error code: {0}")]
    Failed(i32),
    #[error("The async operation failed: {0:?}")]
    Error(winrt::Error),
}

impl From<winrt::Error> for AsyncOperationError {
    fn from(err: winrt::Error) -> Self {
        AsyncOperationError::Error(err)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error calling WinRT API: {0:?}")]
    WinRT(winrt::Error),
    #[error("An async operation failed: {0}")]
    AsyncOperation(#[from] AsyncOperationError)
}

impl From<winrt::Error> for Error {
    fn from(err: winrt::Error) -> Self {
        Error::WinRT(err)
    }
}