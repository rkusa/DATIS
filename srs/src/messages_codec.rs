use crate::message::Message;
use bytes::BytesMut;
use std::{error, fmt, io};
use tokio_codec::{Decoder, Encoder, LinesCodec};

pub struct MessagesCodec {
    lines_codec: LinesCodec,
}

impl MessagesCodec {
    pub fn new() -> Self {
        MessagesCodec {
            lines_codec: LinesCodec::new(),
        }
    }
}

impl Decoder for MessagesCodec {
    type Item = Message;
    type Error = MessagesCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(line) = self.lines_codec.decode(buf)? {
            let msg = serde_json::from_str(&line)?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(line) = self.lines_codec.decode_eof(buf)? {
            let msg = serde_json::from_str(&line)?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for MessagesCodec {
    type Item = Message;
    type Error = MessagesCodecError;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let json = serde_json::to_string(&msg)?;
        self.lines_codec.encode(json, buf)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum MessagesCodecError {
    Json(serde_json::Error),
    LinesCodec(tokio_codec::LinesCodecError),
    Io(io::Error),
}

impl fmt::Display for MessagesCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessagesCodecError::Json(_) => write!(f, "failed to encode/decode JSON"),
            MessagesCodecError::LinesCodec(err) => err.fmt(f),
            MessagesCodecError::Io(err) => err.fmt(f),
        }
    }
}

impl error::Error for MessagesCodecError {}

impl From<io::Error> for MessagesCodecError {
    fn from(err: io::Error) -> Self {
        MessagesCodecError::Io(err)
    }
}

impl From<serde_json::Error> for MessagesCodecError {
    fn from(err: serde_json::Error) -> Self {
        MessagesCodecError::Json(err)
    }
}

impl From<tokio_codec::LinesCodecError> for MessagesCodecError {
    fn from(err: tokio_codec::LinesCodecError) -> Self {
        MessagesCodecError::LinesCodec(err)
    }
}
