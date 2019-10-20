#[macro_use]
extern crate log;

mod client;
mod message;
mod messages_codec;
mod voice_codec;

use client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .unwrap();

    let addr = "89.163.144.74:5026";
    debug!("Connection to {}", addr);
    let client = Client::new("Test", 251_000_000);
    client.start(addr).await?;

    Ok(())
}
