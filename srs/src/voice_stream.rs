use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::client::Client;
use crate::message::{Client as MsgClient, Coalition, Message, MsgType, Radio, RadioInfo};
use crate::messages_codec::{MessagesCodec, MessagesCodecError};
use crate::voice_codec::*;
use futures::future::{self, FutureExt};
use futures::sink::Sink;
use futures::stream::{Stream, StreamExt};
use futures_util::sink::SinkExt;
use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::{TcpStream, UdpFramed, UdpSocket};
use tokio::timer::delay_for;
use tokio_codec::Framed;

const SRS_VERSION: &str = "1.7.0.0";

pub struct VoiceStream {
    addr: SocketAddr,
    voice_stream: UdpFramed<VoiceCodec>,
    heartbeat: Pin<Box<dyn Send + Future<Output = Result<((), ()), MessagesCodecError>>>>,
    client: Client,
    packet_id: u64,
}

impl VoiceStream {
    pub async fn new(client: Client, addr: SocketAddr) -> Result<Self, io::Error> {
        let tcp = TcpStream::connect(addr).await?;
        let (sink, stream) = Framed::new(tcp, MessagesCodec::new()).split();

        let udp = UdpSocket::bind("127.0.0.1:0").await?;
        udp.connect(addr).await?;
        let voice_stream = UdpFramed::new(udp, VoiceCodec::new());

        let heartbeat = future::try_join(recv_updates(stream), send_updates(client.clone(), sink));

        Ok(VoiceStream {
            addr,
            voice_stream,
            heartbeat: Box::pin(heartbeat),
            client,
            packet_id: 1,
        })
    }
}

impl Stream for VoiceStream {
    type Item = Result<VoicePacket, anyhow::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let s = self.get_mut();

        match s.voice_stream.poll_next_unpin(cx) {
            Poll::Pending => {}
            Poll::Ready(None) => {
                return Poll::Ready(Some(Err(anyhow!("voice stream was closed unexpectedly"))))
            }
            Poll::Ready(Some(Ok((p, _)))) => return Poll::Ready(Some(Ok(p))),
            Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err.into()))),
        }

        match s.heartbeat.poll_unpin(cx) {
            Poll::Pending => {}
            Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err.into()))),
            Poll::Ready(Ok(_)) => {
                return Poll::Ready(Some(Err(anyhow!("TCP connection was closed unexpectedly"))))
            }
        }

        Poll::Pending
    }
}

impl Sink<Vec<u8>> for VoiceStream {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let s = self.get_mut();
        Pin::new(&mut s.voice_stream).poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        let mut sguid = [0; 22];
        sguid.clone_from_slice(self.client.sguid.as_bytes());

        let packet = VoicePacket {
            audio_part: item,
            frequencies: vec![Frequency {
                freq: self.client.freq as f64,
                modulation: Modulation::AM,
                encryption: Encryption::None,
            }],
            unit_id: self.client.unit.as_ref().map(|u| u.id).unwrap_or(0),
            packet_id: self.packet_id,
            sguid: sguid,
        };

        let s = self.get_mut();
        s.packet_id = s.packet_id.wrapping_add(1);

        Pin::new(&mut s.voice_stream)
            .start_send((packet, s.addr))
            .map_err(|err| err.into())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let s = self.get_mut();
        Pin::new(&mut s.voice_stream).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let s = self.get_mut();
        Pin::new(&mut s.voice_stream).poll_close(cx)
    }
}

async fn recv_updates(
    mut stream: SplitStream<Framed<TcpStream, MessagesCodec>>,
) -> Result<(), MessagesCodecError> {
    while let Some(_) = stream.next().await {
        // discard messages for now
    }

    Ok(())
}

async fn send_updates(
    client: Client,
    mut sink: SplitSink<Framed<TcpStream, MessagesCodec>, Message>,
) -> Result<(), MessagesCodecError> {
    // send initial SYNC message
    debug!("Sending sync message");

    let sync_msg = create_sync_message(&client);
    sink.send(sync_msg).await?;

    loop {
        delay_for(Duration::from_secs(5)).await;

        debug!("Sending update message");
        let upd_msg = create_update_message(&client);
        sink.send(upd_msg).await?;
    }
}

fn create_sync_message(client: &Client) -> Message {
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid.clone(),
            name: client.name.clone(),
            position: client.pos.clone(),
            coalition: Coalition::Blue,
            radio_info: Some(RadioInfo {
                name: "DATIS Radios".to_string(),
                pos: client.pos.clone(),
                ptt: false,
                radios: vec![Radio {
                    enc: false,
                    enc_key: 0,
                    enc_mode: 0, // no encryption
                    freq_max: 1.0,
                    freq_min: 1.0,
                    freq: client.freq as f64,
                    modulation: 0,
                    name: "DATIS Radio".to_string(),
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
                unit: client
                    .unit
                    .as_ref()
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| client.name.clone()),
                unit_id: client.unit.as_ref().map(|u| u.id).unwrap_or(0),
                simultaneous_transmission: true,
            }),
        }),
        msg_type: MsgType::Sync,
        version: SRS_VERSION.to_string(),
    }
}

fn create_update_message(client: &Client) -> Message {
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid.clone(),
            name: client.name.clone(),
            position: client.pos.clone(),
            coalition: Coalition::Blue,
            radio_info: None,
        }),
        msg_type: MsgType::Update,
        version: SRS_VERSION.to_string(),
    }
}
