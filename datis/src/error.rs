use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    Lua(::hlua51::LuaError),
    LuaFunctionCall(::hlua51::LuaFunctionCallError<::hlua51::Void>),
    // TODO: improve by including information about the global/key that was not defined
    Undefined(std::option::NoneError),
    Tcp(std::io::Error),
    Json(serde_json::error::Error),
    Request(reqwest::Error),
    Base64Decode(base64::DecodeError),
    Ogg(ogg::reading::OggReadError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::error::Error;

        write!(f, "Error: {}", self.description())?;
        let mut cause: Option<&dyn error::Error> = self.cause();
        while let Some(err) = cause {
            write!(f, "  -> {}", err)?;
            cause = err.cause();
        }

        Ok(())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        use self::Error::*;

        match *self {
            Lua(_) => "Lua error",
            LuaFunctionCall(_) => "Error calling Lua function",
            Undefined(_) => "Trying to access lua gobal or table key that does not exist",
            Tcp(_) => "Error establishing TCP connection to SRS",
            Json(_) => "Error serializing/deserializing JSON RPC message",
            Request(_) => "Error sending TTS request",
            Base64Decode(_) => "Error decoding TTS audio content",
            Ogg(_) => "Error decoding OGG audio stream",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        use self::Error::*;

        match *self {
            Lua(ref err) => Some(err),
            LuaFunctionCall(ref err) => Some(err),
            Tcp(ref err) => Some(err),
            Json(ref err) => Some(err),
            Request(ref err) => Some(err),
            Base64Decode(ref err) => Some(err),
            Ogg(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<::hlua51::LuaError> for Error {
    fn from(err: ::hlua51::LuaError) -> Self {
        Error::Lua(err)
    }
}

impl From<::hlua51::LuaFunctionCallError<::hlua51::Void>> for Error {
    fn from(err: ::hlua51::LuaFunctionCallError<::hlua51::Void>) -> Self {
        Error::LuaFunctionCall(err)
    }
}

impl From<std::option::NoneError> for Error {
    fn from(err: std::option::NoneError) -> Self {
        Error::Undefined(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Tcp(err)
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
