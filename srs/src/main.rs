#[macro_use]
extern crate log;

mod message;
mod messages_codec;
mod voice_codec;

use std::time::Duration;

use futures::join;
use futures_util::sink::SinkExt;
use futures_util::stream::{SplitSink, SplitStream, StreamExt};
use message::*;
use messages_codec::MessagesCodec;
use tokio::net::TcpStream;
use tokio::timer::delay_for;
use tokio_codec::Framed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .unwrap();

    let ip = "89.163.144.74:5026";
    debug!("Connection to {}", ip);
    let conn = TcpStream::connect(ip).await?;
    let (sink, stream) = Framed::new(conn, MessagesCodec::new()).split();

    let (a, b) = join!(tx_part(sink), rx_part(stream));
    a?;
    b?;

    Ok(())
}

async fn rx_part(
    mut stream: SplitStream<Framed<TcpStream, MessagesCodec>>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Awaiting data");
    while let Some(asd) = stream.next().await {
        dbg!(asd);
    }

    Ok(())
}

async fn tx_part(
    mut sink: SplitSink<Framed<TcpStream, MessagesCodec>, Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    // send initial SYNC message
    debug!("Sending sync message");

    let sguid = create_sguid();
    let position = Position::default();
    let sync_msg = Message {
        client: Some(Client {
            client_guid: sguid.clone(),
            name: "Test".to_string(),
            position: position.clone(),
            coalition: Coalition::Blue,
            radio_info: Some(RadioInfo {
                name: "Test".to_string(),
                pos: position,
                ptt: false,
                radios: vec![Radio {
                    enc: false,
                    enc_key: 0,
                    enc_mode: 0, // no encryption
                    freq_max: 1.0,
                    freq_min: 1.0,
                    freq: 251_000_000.0,
                    modulation: 0,
                    name: "Test".to_string(),
                    sec_freq: 0.0,
                    volume: 1.0,
                    freq_mode: 0, // Cockpit
                    vol_mode: 0,  // Cockpit
                    expansion: false,
                    channel: -1,
                    simul: false,
                }],
                control: 0, // HOTAS
                selected: 0,
                unit: "Test".to_string(),
                unit_id: 0,
                simultaneous_transmission: true,
            }),
        }),
        msg_type: MsgType::Sync,
        version: "1.7.0.0".to_string(),
    };
    sink.send(sync_msg).await?;

    loop {
        delay_for(Duration::from_secs(5)).await;

        debug!("Sending update message");
        let upd_msg = Message {
            client: Some(Client {
                client_guid: sguid.clone(),
                name: "Test".to_string(),
                position: Position::default(),
                coalition: Coalition::Blue,
                radio_info: None,
            }),
            msg_type: MsgType::Update,
            version: "1.7.0.0".to_string(),
        };
        sink.send(upd_msg).await?;
    }

    // Ok(())
}
