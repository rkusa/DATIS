use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::client::Client;
use crate::message::{Client as MsgClient, Coalition, Message, MsgType, Radio, RadioInfo};
use crate::messages_codec::MessagesCodec;
use crate::voice_codec::*;
use futures::channel::mpsc;
use futures::future::{self, Either, FutureExt, TryFutureExt};
use futures::sink::Sink;
use futures::stream::{Stream, StreamExt};
use futures_util::sink::SinkExt;
use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::{TcpStream, UdpFramed, UdpSocket};
use tokio::timer::delay_for;
use tokio_codec::Framed;

const SRS_VERSION: &str = "1.7.0.0";

pub struct VoiceStream {
    voice_sink: mpsc::Sender<Packet>,
    voice_stream: SplitStream<UdpFramed<VoiceCodec>>,
    heartbeat: Pin<Box<dyn Send + Future<Output = Result<(), anyhow::Error>>>>,
    client: Client,
    packet_id: u64,
}

impl VoiceStream {
    pub async fn new(
        client: Client,
        addr: SocketAddr,
        recv_voice: bool,
    ) -> Result<Self, io::Error> {
        let tcp = TcpStream::connect(addr).await?;
        let (sink, stream) = Framed::new(tcp, MessagesCodec::new()).split();

        let a = Box::pin(recv_updates(stream));
        let b = Box::pin(send_updates(client.clone(), sink, recv_voice));

        let udp = UdpSocket::bind("127.0.0.1:0").await?;
        udp.connect(addr).await?;
        let (sink, stream) = UdpFramed::new(udp, VoiceCodec::new()).split();
        let (tx, rx) = mpsc::channel(32);

        let c = Box::pin(send_voice_pings(client.clone(), tx.clone(), recv_voice));
        let d = Box::pin(forward_packets(rx, sink, addr));

        let ab = future::try_select(a, b)
            .map_ok(|_| ())
            .map_err(|err| match err {
                Either::Left((err, _)) => err,
                Either::Right((err, _)) => err,
            });
        let cd = future::try_select(c, d)
            .map_ok(|_| ())
            .map_err(|err| match err {
                Either::Left((err, _)) => err,
                Either::Right((err, _)) => err,
            });
        let heartbeat = future::try_select(ab, cd)
            .map_ok(|_| ())
            .map_err(|err| match err {
                Either::Left((err, _)) => err,
                Either::Right((err, _)) => err,
            });

        Ok(VoiceStream {
            voice_stream: stream,
            voice_sink: tx,
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
            Poll::Ready(Some(Ok((None, _)))) => {
                // not enough data for the codec to create a new item
            }
            Poll::Ready(Some(Ok((Some(p), _)))) => {
                return Poll::Ready(Some(Ok(p)));
            }
            Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err.into()))),
        }

        match s.heartbeat.poll_unpin(cx) {
            Poll::Pending => {}
            Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err.into()))),
            Poll::Ready(Ok(asd)) => {
                debug!("WTF {:?}", asd);
                return Poll::Ready(Some(Err(anyhow!("TCP connection was closed unexpectedly"))));
            }
        }

        Poll::Pending
    }
}

impl Sink<Vec<u8>> for VoiceStream {
    type Error = mpsc::SendError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let s = self.get_mut();
        Pin::new(&mut s.voice_sink).poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        let mut sguid = [0; 22];
        sguid.clone_from_slice(self.client.sguid().as_bytes());

        let packet = VoicePacket {
            audio_part: item,
            frequencies: vec![Frequency {
                freq: self.client.freq() as f64,
                modulation: Modulation::AM,
                encryption: Encryption::None,
            }],
            unit_id: self.client.unit().map(|u| u.id).unwrap_or(0),
            packet_id: self.packet_id,
            sguid,
        };

        let s = self.get_mut();
        s.packet_id = s.packet_id.wrapping_add(1);

        Pin::new(&mut s.voice_sink).start_send(packet.into())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let s = self.get_mut();
        Pin::new(&mut s.voice_sink).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let s = self.get_mut();
        Pin::new(&mut s.voice_sink).poll_close(cx)
    }
}

async fn recv_updates(
    mut stream: SplitStream<Framed<TcpStream, MessagesCodec>>,
) -> Result<(), anyhow::Error> {
    while let Some(msg) = stream.next().await {
        // discard messages for now
        msg?;
    }

    Ok(())
}

async fn send_updates(
    client: Client,
    mut sink: SplitSink<Framed<TcpStream, MessagesCodec>, Message>,
    recv_voice: bool,
) -> Result<(), anyhow::Error> {
    // send initial SYNC message
    let sync_msg = create_sync_message(&client);
    sink.send(sync_msg).await?;

    loop {
        delay_for(Duration::from_secs(5)).await;

        // Sending update message
        let upd_msg = if recv_voice {
            // to recv audio we have to update our radio info regularly
            create_radio_update_message(&client)
        } else {
            create_update_message(&client)
        };
        sink.send(upd_msg).await?;
    }
}

async fn send_voice_pings(
    client: Client,
    mut tx: mpsc::Sender<Packet>,
    recv_voice: bool,
) -> Result<(), anyhow::Error> {
    // TODO: is there a future that never resolves
    let mut sguid = [0; 22];
    sguid.clone_from_slice(client.sguid().as_bytes());

    loop {
        if recv_voice {
            tx.send(Packet::Ping(sguid.clone())).await?;
        }

        delay_for(Duration::from_secs(5)).await;
    }
}

async fn forward_packets(
    mut rx: mpsc::Receiver<Packet>,
    mut sink: SplitSink<UdpFramed<VoiceCodec>, (Packet, SocketAddr)>,
    addr: SocketAddr,
) -> Result<(), anyhow::Error> {
    while let Some(p) = rx.next().await {
        sink.send((p, addr)).await?;
    }

    Ok(())
}

fn create_sync_message(client: &Client) -> Message {
    let pos = client.position();
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(client.name().to_string()),
            position: pos.clone(),
            coalition: Coalition::Blue,
            radio_info: Some(RadioInfo {
                name: "DATIS Radios".to_string(),
                pos: pos,
                ptt: false,
                radios: vec![Radio {
                    enc: false,
                    enc_key: 0,
                    enc_mode: 0, // no encryption
                    freq_max: 1.0,
                    freq_min: 1.0,
                    freq: client.freq() as f64,
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
                    .unit()
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| client.name().to_string()),
                unit_id: client.unit().as_ref().map(|u| u.id).unwrap_or(0),
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
            client_guid: client.sguid().to_string(),
            name: Some(client.name().to_string()),
            position: client.position(),
            coalition: Coalition::Blue,
            radio_info: None,
        }),
        msg_type: MsgType::Update,
        version: SRS_VERSION.to_string(),
    }
}

fn create_radio_update_message(client: &Client) -> Message {
    let pos = client.position();
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(client.name().to_string()),
            position: pos.clone(),
            coalition: Coalition::Blue,
            radio_info: Some(RadioInfo {
                name: "DATIS Radios".to_string(),
                pos: pos,
                ptt: false,
                radios: vec![Radio {
                    enc: false,
                    enc_key: 0,
                    enc_mode: 0, // no encryption
                    freq_max: 1.0,
                    freq_min: 1.0,
                    freq: client.freq() as f64,
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
                    .unit()
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| client.name().to_string()),
                unit_id: client.unit().map(|u| u.id).unwrap_or(0),
                simultaneous_transmission: true,
            }),
        }),
        msg_type: MsgType::RadioUpdate,
        version: SRS_VERSION.to_string(),
    }
}
