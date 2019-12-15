#[macro_use]
extern crate log;

#[macro_use]
extern crate anyhow;


use std::str::FromStr;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::time::Duration;

use futures::prelude::*;

use clap::{App, Arg};
use srs::{Client, VoiceStream};
use dotenv::dotenv;
use futures::future::{Either, FutureExt};
use futures_util::stream::{SplitSink, SplitStream, StreamExt};

use tokio::timer::delay_for;

use tokio;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .unwrap();

    let frequency = 244000000;
    let client = Client::new("thezoq2_srsrstest", frequency);

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(64,94,100,43)), 5002);

    let (sink, stream) = client.start(addr, true).await?.split();

    let rx = Box::pin(recv_voice_packets(stream));
    let tx = Box::pin(audio_broadcast(sink));

    match future::try_select(rx, tx).await {
        Err(Either::Left((err, _))) => Err(err.into()),
        Err(Either::Right((err, _))) => Err(err.into()),
        _ => Ok(()),
    }
}


async fn recv_voice_packets(mut stream: SplitStream<VoiceStream>) -> Result<(), anyhow::Error> {
    while let Some(packet) = stream.next().await {
        packet.expect("Voice packet receive error");
        // we are currently not interested in the received voice packets, so simply discard them
        println!("Got packet");
    }

    println!("Warning: Got out of recv_voice_packets loop");

    Ok(())
}

async fn audio_broadcast(
    mut sink: SplitSink<VoiceStream, Vec<u8>>,
) -> Result<(), anyhow::Error> {
    loop {
        delay_for(Duration::from_secs(5)).await;
        println!("Broadcast thread idling")
    }
}
