#![recursion_limit = "512"]

mod client;
pub mod message;
mod messages_codec;
mod voice_codec;
mod voice_stream;

pub use client::Client;
pub use voice_codec::{Encryption, Frequency, Modulation, VoicePacket};
pub use voice_stream::VoiceStream;
