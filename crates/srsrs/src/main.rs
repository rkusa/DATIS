#[macro_use]
extern crate log;

#[macro_use]
extern crate anyhow;


use std::str::FromStr;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::time::Duration;
use std::io::prelude::*;
use std::collections::HashMap;


use clap::{App, Arg};
use dotenv::dotenv;
use futures::prelude::*;
use futures::future::{Either, FutureExt};
use futures::channel::mpsc;
use futures_util::stream::{SplitSink, SplitStream, StreamExt};
use tokio;
use tokio::timer::delay_for;
use audiopus::{coder::Decoder, Channels, SampleRate};
use rodio::{Source, Sink};

use srs::{
    Client,
    VoiceStream,
    VoicePacket,
    message::{GameRadio, GameMessage},
};

mod dcs_control;


async fn split_channel<T: Clone>(
    mut rx: mpsc::Receiver<T>,
    mut tx1: mpsc::UnboundedSender<T>,
    mut tx2: mpsc::UnboundedSender<T>
) -> Result<(), anyhow::Error>{
    loop {
        let received = rx.next().await
            .ok_or(anyhow!("Sender disconnected"))?;
        tx1.send(received.clone()).await?;
        tx2.send(received.clone()).await?;
    }
}


#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .unwrap();

    let frequency = 245000000;
    let client = Client::new("thezoq2_srsrstest", frequency);

    let (game_tx, split_rx) = mpsc::channel(20);
    let (split_tx1, game_rx) = mpsc::unbounded();
    let (split_tx2, radio_rx) = mpsc::unbounded();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(64,94,100,43)), 5002);

    let (sink, stream) = client.start(addr, game_rx, true).await?.split();

    let control = Box::pin(dcs_control::dcs_control(game_tx));
    let channel_splter = Box::pin(split_channel(split_rx, split_tx1, split_tx2));

    let rx = Box::pin(recv_voice_packets(stream, radio_rx));
    let tx = Box::pin(audio_broadcast(sink));

    let control_rx = future::try_select(control, rx)
        .map_ok(|_| ())
        .map_err(|err| match err {
            Either::Left((err, _)) => err,
            Either::Right((err, _)) => err,
        });

    let split_control_rx = future::try_select(channel_splter, control_rx)
        .map_ok(|_| ())
        .map_err(|err| match err {
            Either::Left((err, _)) => err,
            Either::Right((err, _)) => err,
        });

    match future::try_select(split_control_rx, tx).await {
        Err(Either::Left((err, _))) => Err(err.into()),
        Err(Either::Right((err, _))) => Err(err.into()),
        _ => Ok(()),
    }
}


async fn recv_voice_packets(
    mut stream: SplitStream<VoiceStream>,
    mut game_info: mpsc::UnboundedReceiver<srs::message::GameMessage>
)
    -> Result<(), anyhow::Error>
{
    let mut dec = Decoder::new(SampleRate::Hz16000, Channels::Mono)
        .expect("Failed to create decoder");
    let mut output = [0i16; 2048];

    let device = rodio::default_output_device().unwrap();

    let mut sinks = HashMap::new();

    let mut radios: Option<Vec<srs::message::GameRadio>> = None;

    println!("Got past sine wave");

    loop {
        let next = future::select(stream.next(), game_info.next()).await;
        match next {
            Either::Left((Some(packet), _)) => {
                let packet = packet
                    .expect("Voice packet receive error");

                if !sinks.contains_key(&packet.sguid) {
                    sinks.insert(packet.sguid, Sink::new(&device));
                }

                let decode_result = dec.decode(
                    Some(&packet.audio_part),
                    &mut output[..],
                    false
                );

                match decode_result {
                    Ok(len) => {
                        let buffer = rodio::buffer::SamplesBuffer::new(
                            1,
                            16000,
                            &output[0..len]
                        );
                        // println!("{:?}", packet.sguid);
                        let sink = &sinks[&packet.sguid];
                        sink.set_volume(packet_volume(&packet, &radios));
                        sink.append(buffer);
                    },
                    Err(e) => {println!("Decoder error: {:?}", e)}
                }
            }
            Either::Right((Some(message), _)) => {
                radios = Some(message.radios);
            }
            Either::Left((None, _)) => {
                panic!("No more voice messages");
            }
            Either::Right((None, _)) => {
                panic!("No more game messages");
            }
        }
    }
}

async fn audio_broadcast(
    mut sink: SplitSink<VoiceStream, Vec<u8>>,
) -> Result<(), anyhow::Error> {
    loop {
        delay_for(Duration::from_secs(5)).await;
        println!("Broadcast thread idling")
    }
}


fn packet_volume(packet: &VoicePacket, radios: &Option<Vec<GameRadio>>) -> f32 {
    radios.as_ref().map(|radios| {
            packet.frequencies.iter().map(|freq| {
                radios.iter()
                    .filter(|radio| radio.freq == freq.freq as f64)
                    .map(|radio| radio.volume)
                    .next()
                    .unwrap_or(1.0)
            })
            .next()
            .unwrap_or(1.0)
        })
        .unwrap_or(1.0)
}
