use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::client::Client;
use crate::message::{
    Client as MsgClient, Coalition, GameMessage, Message, MsgType, Radio, RadioInfo,
    RadioSwitchControls,
};
use crate::messages_codec::MessagesCodec;
use crate::voice_codec::*;
use futures::channel::mpsc;
use futures::future::{self, Either, FutureExt, TryFutureExt};
use futures::sink::{Sink, SinkExt};
use futures::stream::{SplitSink, SplitStream, Stream, StreamExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::delay_for;
use tokio_util::codec::Framed;
use tokio_util::udp::UdpFramed;

const SRS_VERSION: &str = "1.8.0.0";

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
        game_source: Option<mpsc::UnboundedReceiver<GameMessage>>,
    ) -> Result<Self, io::Error> {
        let recv_voice = game_source.is_some();

        let tcp = TcpStream::connect(addr).await?;
        let (sink, stream) = Framed::new(tcp, MessagesCodec::new()).split();

        let a = Box::pin(recv_updates(stream));
        let b = Box::pin(send_updates(client.clone(), sink, game_source));

        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let udp = UdpSocket::bind(local_addr).await?;
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
            Poll::Ready(Ok(_)) => {
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

/// Sends updates about the client to the server. If `game_source` is set,
/// the position and frequency from the latest received `GameMessage` is used.
/// Otherwise, the parameters set in the `client` struct are used.
async fn send_updates<G>(
    client: Client,
    mut sink: SplitSink<Framed<TcpStream, MessagesCodec>, Message>,
    game_source: Option<G>,
) -> Result<(), anyhow::Error>
where
    G: Stream<Item = GameMessage> + Unpin,
{
    // send initial Update message
    let msg = create_radio_update_message(&client);
    sink.send(msg).await?;

    if let Some(mut game_source) = game_source {
        let mut last_game_msg = None;

        loop {
            let delay = delay_for(Duration::from_secs(5));
            match future::select(game_source.next(), delay).await {
                Either::Left((Some(msg), _)) => {
                    last_game_msg = Some(msg);
                }
                Either::Left((None, _)) => {
                    break;
                }
                Either::Right((_, _)) => {
                    // debug!("Game message timeout")
                }
            }

            match &last_game_msg {
                Some(msg) => sink.send(radio_message_from_game(&client, msg)).await?,
                None => {}
            }
        }

        log::warn!("Game source disconnected");

        Ok(())
    } else {
        let mut old_pos = client.position();
        loop {
            delay_for(Duration::from_secs(60)).await;

            // keep the position of the station updated
            let new_pos = client.position();
            if new_pos != old_pos {
                log::debug!(
                    "Position of {} changed, sending a new update message",
                    client.name()
                );
                sink.send(create_update_message(&client)).await?;
                old_pos = new_pos;
            }
        }
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

fn create_radio_update_message(client: &Client) -> Message {
    let pos = client.position();
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(client.name().to_string()),
            coalition: Coalition::Blue,
            radio_info: Some(RadioInfo {
                name: "DATIS Radios".to_string(),
                ptt: false,
                // TODO: enable one of the radios to receive voice
                radios: std::iter::repeat_with(Radio::default).take(10).collect(),
                control: crate::message::RadioSwitchControls::Hotas,
                selected: 0,
                unit: client
                    .unit()
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| client.name().to_string()),
                unit_id: client.unit().as_ref().map(|u| u.id).unwrap_or(0),
                simultaneous_transmission: true,
            }),
            lat_lng_position: Some(pos.clone()),
        }),
        msg_type: MsgType::RadioUpdate,
        version: SRS_VERSION.to_string(),
    }
}

fn create_update_message(client: &Client) -> Message {
    let pos = client.position();
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(client.name().to_string()),
            coalition: Coalition::Blue,
            radio_info: None,
            lat_lng_position: Some(pos.clone()),
        }),
        msg_type: MsgType::Update,
        version: SRS_VERSION.to_string(),
    }
}

fn radio_message_from_game(client: &Client, game_message: &GameMessage) -> Message {
    let pos = game_message.lat_lng_position.clone();

    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(game_message.name.clone()),
            coalition: Coalition::Blue,
            radio_info: Some(RadioInfo {
                name: game_message.name.clone(),
                ptt: game_message.ptt,
                radios: game_message.radios.clone(),
                control: RadioSwitchControls::Hotas,
                selected: game_message.selected,
                unit: game_message.unit.clone(),
                unit_id: game_message.unit_id,
                simultaneous_transmission: true,
            }),
            lat_lng_position: Some(pos.clone()),
        }),
        msg_type: MsgType::RadioUpdate,
        version: SRS_VERSION.to_string(),
    }
}
