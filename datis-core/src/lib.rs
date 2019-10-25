#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde;

mod error;
pub mod export;
mod srs;
pub mod station;
pub mod tts;
mod utils;
pub mod weather;
mod worker;

pub use srs::AtisSrsClient;
