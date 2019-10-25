use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    // TODO: improve by including information about the global/key that was not defined
    Undefined(String),
    Io(std::io::Error),
    Json(serde_json::error::Error),
    Request(reqwest::Error),
    Base64Decode(base64::DecodeError),
    Ogg(ogg::reading::OggReadError),
    GcloudTTL(serde_json::Value),
    PollyTTS(String),
    Weather(Box<dyn error::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Error::*;
        use std::error::Error;

        match self {
            Undefined(key) => write!(
                f,
                "Error: Trying to access undefined lua global or table key: {}",
                key
            )?,
            GcloudTTL(json) => {
                write!(f, "Error calling Gcloud TTS service: {}", json.to_string(),)?
            }
            _ => write!(f, "Error: {}", self.description())?,
        }

        let mut cause: Option<&dyn error::Error> = self.source();
        while let Some(err) = cause {
            write!(f, "  -> {}", err)?;
            cause = err.source();
        }

        Ok(())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        use self::Error::*;

        match *self {
            Undefined(_) => "Trying to access lua gobal or table key that does not exist",
            Io(_) => "Error connecting/sending data to SRS",
            Json(_) => "Error serializing/deserializing JSON RPC message",
            Request(_) => "Error sending TTS request",
            Base64Decode(_) => "Error decoding TTS audio content",
            Ogg(_) => "Error decoding OGG audio stream",
            GcloudTTL(_) => "Error calling Gcloud TTS service",
            PollyTTS(_) => "Error calling Amazon Polly service",
            Weather(_) => "Error getting current weather",
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use self::Error::*;

        match *self {
            Io(ref err) => Some(err),
            Json(ref err) => Some(err),
            Request(ref err) => Some(err),
            Base64Decode(ref err) => Some(err),
            Ogg(ref err) => Some(err),
            Weather(ref err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Error::Json(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Request(err)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(err: base64::DecodeError) -> Self {
        Error::Base64Decode(err)
    }
}

impl From<ogg::reading::OggReadError> for Error {
    fn from(err: ogg::reading::OggReadError) -> Self {
        Error::Ogg(err)
    }
}
