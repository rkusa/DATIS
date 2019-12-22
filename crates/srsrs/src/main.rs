#[macro_use]
extern crate log;

#[macro_use]
extern crate anyhow;


use std::net::ToSocketAddrs;
use std::time::Duration;

use clap;


use dotenv::dotenv;
use futures::prelude::*;
use futures::future::Either;
use futures::channel::mpsc;
use futures_util::stream::{SplitSink, SplitStream};
use tokio;
use tokio::timer::delay_for;
use audiopus::{coder::Decoder, Channels, SampleRate};
use rodio::{Sink, source::ChannelVolume};

use srs::{
    Client,
    VoiceStream,
    VoicePacket,
    message::{GameRadio},
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


    let matches = clap::App::new("srsrs")
        .version("1.0")
        .arg(clap::Arg::with_name("SERVER")
            .index(1)
            .required(true))
        .arg(clap::Arg::with_name("PORT")
            .index(2)
            .required(false))
        .get_matches();

    let server = matches.value_of("SERVER").unwrap();
    let port = matches.value_of("PORT").unwrap_or("5002");

    let frequency = 245000000;
    let client = Client::new("thezoq2_srsrstest", frequency);

    let (game_tx, split_rx) = mpsc::channel(20);
    let (split_tx1, game_rx) = mpsc::unbounded();
    let (split_tx2, radio_rx) = mpsc::unbounded();

    // let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(136,55,80,214)), 5002);
    let mut addr = format!("{}:{}", server, port).to_socket_addrs().unwrap();

    let (sink, stream) = client.start(addr.next().unwrap(), Some(game_rx)).await?.split();

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
    let mut output = [0i16; 1024];

    let device = rodio::default_output_device().unwrap();

    // let mut sinks = HashMap::new();
    let mut sinks: Vec<([u8; 22], Decoder, rodio::Sink)> = vec!();

    let mut radios: Option<Vec<srs::message::GameRadio>> = None;

    println!("Got past sine wave");


    loop {
        let next = future::select(stream.next(), game_info.next()).await;
        match next {
            Either::Left((Some(packet), _)) => {
                let packet = packet
                    .expect("Voice packet receive error");

                let mut sink_index = None;
                for (i, (id, _, _)) in sinks.iter().enumerate() {
                    if id == &packet.sguid {
                        sink_index = Some(i);
                    }
                }
                if sink_index == None {
                    let dec = Decoder::new(SampleRate::Hz16000, Channels::Mono)
                        .expect("Failed to create decoder");
                    sinks.push((packet.sguid, dec, Sink::new(&device)));
                }
                let sink_index = sink_index.unwrap_or(sinks.len()-1);
                let (_, dec, sink) = &mut sinks[sink_index];

                let decode_result = dec.decode(
                    Some(&packet.audio_part),
                    &mut output[..],
                    false
                );

                match decode_result {
                    Ok(len) => {
                        // println!("{:?}", packet.sguid);
                        sink.set_volume(packet_volume(&packet, &radios));
                        let source = rodio::buffer::SamplesBuffer::new(
                            1,
                            16000,
                            &output[0..len]
                        );
                        let with_channel = ChannelVolume::new(
                            source,
                            packet_channels(&packet, &radios)
                        );
                        sink.append(with_channel);
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
    _sink: SplitSink<VoiceStream, Vec<u8>>,
) -> Result<(), anyhow::Error> {
    loop {
        delay_for(Duration::from_secs(5)).await;
        println!("Broadcast thread idling")
    }
}


fn packet_volume(packet: &VoicePacket, radios: &Option<Vec<GameRadio>>) -> f32 {
    packet_radio(packet, radios)
        .map(|(_, r)| r.volume)
        .unwrap_or(1.0)
}
fn packet_channels(packet: &VoicePacket, radios: &Option<Vec<GameRadio>>)
    -> Vec<f32>
{
    packet_radio(packet, radios)
        .map(|(id, _)| {
            if id % 2 == 1 {
                vec![1., 0.]
            }
            else {
                vec![0., 1.]
            }
        })
        .unwrap_or(vec![1., 1.])
}

fn packet_radio(
    packet: &VoicePacket,
    radios: &Option<Vec<GameRadio>>
) -> Option<(usize, GameRadio)> {
    radios.as_ref().map(|radios| {
            packet.frequencies.iter().map(|freq| {
                radios.iter()
                    .cloned()
                    .enumerate()
                    .filter(|(_, radio)| radio.freq == freq.freq as f64)
                    .next()
            })
            .next()
            .unwrap_or(None)
        })
        .unwrap_or(None)
}
