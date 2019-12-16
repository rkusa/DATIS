#[macro_use]
extern crate log;

#[macro_use]
extern crate anyhow;


use std::str::FromStr;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::time::Duration;
use std::io::prelude::*;

use futures::prelude::*;

use clap::{App, Arg};
use srs::{Client, VoiceStream};
use dotenv::dotenv;
use futures::future::{Either, FutureExt};
use futures_util::stream::{SplitSink, SplitStream, StreamExt};

use tokio::timer::delay_for;

use audiopus::{coder::Decoder, Channels, SampleRate};

use tokio;

use rodio::{Source, Sink};

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
    let mut dec = Decoder::new(SampleRate::Hz16000, Channels::Mono)
        .expect("Failed to create decoder");
    let mut output = [0i16; 2048];

    let device = rodio::default_output_device().unwrap();
    let sink = Sink::new(&device);

    let source = rodio::source::SineWave::new(440);
    // sink.append(source);

    while let Some(packet) = stream.next().await {
        let packet = packet.expect("Voice packet receive error");
        // we are currently not interested in the received voice packets, so simply discard them
        println!("Got packet: ");
        println!("Freqs: {:?}", packet.frequencies);
        println!("ID: {:?}", packet.sguid);
        let decode_result = dec.decode(Some(&packet.audio_part), &mut output[..], false);

        match decode_result {
            Ok(len) => {
                println!("Decoded {} bytes", len);
                let buffer = rodio::buffer::SamplesBuffer::new(
                    1,
                    16000,
                    &output[0..len]
                );
                sink.append(buffer);
            },
            Err(e) => {println!("Decoder error: {:?}", e)}
        }
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
