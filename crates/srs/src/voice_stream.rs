use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use futures::channel::mpsc;
use futures::future::FutureExt;
use futures::select;
use futures::sink::{Sink, SinkExt};
use futures::stream::{SplitStream, Stream, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::oneshot::Receiver;
use tokio::time;
use tokio_util::codec::{FramedRead, FramedWrite};
use tokio_util::udp::UdpFramed;

use crate::client::Client;
use crate::message::{
    Client as MsgClient, GameMessage, Message, MsgType, Radio, RadioInfo, RadioSwitchControls,
};
use crate::messages_codec::{self, MessagesCodec};
use crate::voice_codec::*;

const SRS_VERSION: &str = "1.9.0.0";

pub struct VoiceStream {
    voice_sink: mpsc::Sender<Packet>,
    voice_stream: SplitStream<UdpFramed<VoiceCodec>>,
    heartbeat: Pin<Box<dyn Send + Future<Output = Result<(), VoiceStreamError>>>>,
    client: Client,
    packet_id: u64,
}

#[derive(Clone)]
struct ServerSettings(Arc<ServerSettingsInner>);

struct ServerSettingsInner {
    los_enabled: AtomicBool,
    distance_enabled: AtomicBool,
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceStreamError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    MessagesCodec(#[from] messages_codec::MessagesCodecError),
    #[error("{0}")]
    ChannelSend(#[from] futures::channel::mpsc::SendError),
    #[error("Version mismatch between DATIS ({expected}) and the SRS server ({encountered})")]
    VersionMismatch {
        expected: String,
        encountered: String,
    },
    #[error("Voice stream was closed unexpectedly")]
    Closed,
    #[error("TCP connection was closed unexpectedly")]
    ConnectionClosed,
}

impl VoiceStream {
    pub async fn new(
        client: Client,
        addr: SocketAddr,
        game_source: Option<mpsc::UnboundedReceiver<GameMessage>>,
        shutdown_signal: Receiver<()>,
    ) -> Result<Self, VoiceStreamError> {
        let recv_voice = game_source.is_some();

        let tcp = TcpStream::connect(addr).await?;
        let (stream, sink) = tcp.into_split();
        let mut messages_sink = FramedWrite::new(sink, MessagesCodec::new());
        let messages_stream = FramedRead::new(stream, MessagesCodec::new());

        let server_settings = ServerSettings(Arc::new(ServerSettingsInner {
            los_enabled: AtomicBool::new(false),
            distance_enabled: AtomicBool::new(false),
        }));

        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let udp = UdpSocket::bind(local_addr).await?;
        udp.connect(addr).await?;
        let (mut voice_sink, voice_stream) = UdpFramed::new(udp, VoiceCodec::new()).split();
        let (mut tx, mut rx) = mpsc::channel(32);
        let tx2 = tx.clone();

        let client2 = client.clone();
        let heartbeat = async move {
            let mut messages_stream = messages_stream.fuse();

            // send sync message to receive server settings
            messages_sink
                .send(create_sync_message(&client).await)
                .await?;

            // send initial Update message
            messages_sink
                .send(create_radio_update_message(&client).await)
                .await?;

            let mut old_pos = client.position().await;
            let mut position_update_interval = time::interval(Duration::from_secs(60));
            let mut voice_ping_interval = time::interval(Duration::from_secs(5));
            let mut game_source_interval = time::interval(Duration::from_secs(5));
            let mut shutdown_signal = shutdown_signal.fuse();
            let mut last_game_msg = None;
            let (_tx, noop_game_source) = mpsc::unbounded();
            let send_client_position_updates = game_source.is_none();
            let mut game_source = game_source.unwrap_or(noop_game_source);

            let mut sguid = [0; 22];
            sguid.clone_from_slice(client.sguid().as_bytes());

            loop {
                select! {
                    // receive control messages
                    msg = messages_stream.next() => {
                        if let Some(msg) = msg {
                            let msg = msg?;

                            // update server settings
                            if let Some(settings) = msg.server_settings {
                                server_settings.0.los_enabled.store(
                                    settings.get("LOS_ENABLED").map(|s| s.as_str()) == Some("True"),
                                    Ordering::Relaxed,
                                );
                                server_settings.0.distance_enabled.store(
                                    settings.get("DISTANCE_ENABLED").map(|s| s.as_str()) == Some("true"),
                                    Ordering::Relaxed,
                                );
                            }

                            // handle message
                            if msg.msg_type == MsgType::VersionMismatch {
                                return Err(VoiceStreamError::VersionMismatch {
                                    expected: SRS_VERSION.to_string(),
                                    encountered: msg.version,
                                })
                            }
                        } else {
                            log::debug!("Messages stream was closed, closing voice stream");
                            break;
                        }
                    }

                    // Sends updates about the client to the server. If `game_source` is set,
                    // the position and frequency from the latest received `GameMessage` is used.
                    // Otherwise, the parameters set in the `client` struct are used.
                    _ = position_update_interval.tick().fuse() => {
                        if !send_client_position_updates {
                            continue;
                        }

                        // keep the position of the station updated
                        let new_pos = client.position().await;
                        let los_enabled = server_settings.0.los_enabled.load(Ordering::Relaxed);
                        let distance_enabled = server_settings.0.distance_enabled.load(Ordering::Relaxed);
                        if (los_enabled || distance_enabled) && new_pos != old_pos {
                            log::debug!(
                                "Position of {} changed, sending a new update message",
                                client.name()
                            );
                            messages_sink.send(create_update_message(&client).await).await?;
                            old_pos = new_pos;
                        }
                    }

                    msg = game_source.next() => {
                        if let Some(msg) = msg {
                            last_game_msg = Some(msg);
                        }
                    }

                    _ = game_source_interval.tick().fuse() => {
                        if let Some(msg) = &last_game_msg {
                            messages_sink.send(radio_message_from_game(&client, msg)).await?;
                        }
                    }

                    _ = voice_ping_interval.tick().fuse() => {
                        if recv_voice {
                            tx.send(Packet::Ping(sguid)).await?;
                        }
                    }

                    packet = rx.next() => {
                        if let Some(p) = packet  {
                            voice_sink.send((p, addr)).await?;
                        }
                    }

                    _ = shutdown_signal => {
                        messages_sink.into_inner().shutdown().await?;
                        break;
                    }
                }
            }

            Ok(())
        };

        Ok(VoiceStream {
            voice_stream,
            voice_sink: tx2,
            heartbeat: Box::pin(heartbeat),
            client: client2,
            packet_id: 1,
        })
    }
}

impl Stream for VoiceStream {
    type Item = Result<VoicePacket, VoiceStreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let s = self.get_mut();

        match s.voice_stream.poll_next_unpin(cx) {
            Poll::Pending => {}
            Poll::Ready(None) => return Poll::Ready(Some(Err(VoiceStreamError::Closed))),
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
            Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
            Poll::Ready(Ok(_)) => {
                return Poll::Ready(Some(Err(VoiceStreamError::ConnectionClosed)));
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
                modulation: if self.client.freq() <= 87_995_000 {
                    Modulation::Fm
                } else {
                    Modulation::Am
                },
                encryption: Encryption::None,
            }],
            unit_id: self.client.unit().map(|u| u.id).unwrap_or(0),
            packet_id: self.packet_id,
            hop_count: 0,
            transmission_sguid: sguid,
            client_sguid: sguid,
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

impl Sink<VoicePacket> for VoiceStream {
    type Error = mpsc::SendError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let s = self.get_mut();
        Pin::new(&mut s.voice_sink).poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, mut packet: VoicePacket) -> Result<(), Self::Error> {
        let mut sguid = [0; 22];
        sguid.clone_from_slice(self.client.sguid().as_bytes());
        packet.client_sguid = sguid;

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

async fn create_radio_update_message(client: &Client) -> Message {
    let pos = client.position().await;
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(client.name().to_string()),
            coalition: client.coalition,
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
            lat_lng_position: Some(pos),
        }),
        msg_type: MsgType::RadioUpdate,
        server_settings: None,
        version: SRS_VERSION.to_string(),
    }
}

async fn create_update_message(client: &Client) -> Message {
    let pos = client.position().await;
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(client.name().to_string()),
            coalition: client.coalition,
            radio_info: None,
            lat_lng_position: Some(pos),
        }),
        msg_type: MsgType::Update,
        server_settings: None,
        version: SRS_VERSION.to_string(),
    }
}

async fn create_sync_message(client: &Client) -> Message {
    let pos = client.position().await;
    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(client.name().to_string()),
            coalition: client.coalition,
            radio_info: None,
            lat_lng_position: Some(pos),
        }),
        msg_type: MsgType::Sync,
        server_settings: None,
        version: SRS_VERSION.to_string(),
    }
}

fn radio_message_from_game(client: &Client, game_message: &GameMessage) -> Message {
    let pos = game_message.lat_lng.clone();

    Message {
        client: Some(MsgClient {
            client_guid: client.sguid().to_string(),
            name: Some(game_message.name.clone()),
            coalition: client.coalition,
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
            lat_lng_position: Some(pos),
        }),
        msg_type: MsgType::RadioUpdate,
        server_settings: None,
        version: SRS_VERSION.to_string(),
    }
}
