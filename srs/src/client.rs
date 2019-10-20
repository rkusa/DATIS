use std::time::Duration;

use crate::message::{
    create_sguid, Client as MsgClient, Coalition, Message, MsgType, Position, Radio, RadioInfo,
};
use crate::messages_codec::MessagesCodec;
use futures_util::sink::SinkExt;
use futures_util::stream::{SplitSink, SplitStream, StreamExt};
use tokio::net::TcpStream;
use tokio::timer::delay_for;
use tokio_codec::Framed;
use tokio_net::ToSocketAddrs;

const SRS_VERSION: &str = "1.7.0.0";

struct UnitInfo {
    id: u32,
    name: String,
}

pub struct Client {
    sguid: String,
    name: String,
    freq: u64,
    pos: Position,
    unit: Option<UnitInfo>,
}

impl Client {
    pub fn new(name: &str, freq: u64) -> Self {
        Client {
            sguid: create_sguid(),
            name: name.to_string(),
            freq,
            pos: Position::default(),
            unit: None,
        }
    }

    pub fn with_position(mut self, pos: Position) -> Self {
        self.pos = pos;
        self
    }

    pub fn for_unit(mut self, id: u32, name: &str) -> Self {
        self.unit = Some(UnitInfo {
            id,
            name: name.to_string(),
        });
        self
    }

    pub async fn start<A: ToSocketAddrs>(self, addr: A) -> Result<(), Box<dyn std::error::Error>> {
        self.heartbeat(addr).await?;

        Ok(())
    }

    async fn heartbeat<A: ToSocketAddrs>(&self, addr: A) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Connecting to SRS");
        let conn = TcpStream::connect(addr).await?;
        let (sink, stream) = Framed::new(conn, MessagesCodec::new()).split();

        let (a, b) = futures::join!(self.send_updates(sink), self.recv_updates(stream));
        a?;
        b?;

        Ok(())
    }

    async fn recv_updates(
        &self,
        mut stream: SplitStream<Framed<TcpStream, MessagesCodec>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Awaiting data");
        while let Some(asd) = stream.next().await {
            dbg!(asd);
        }

        Ok(())
    }

    async fn send_updates(
        &self,
        mut sink: SplitSink<Framed<TcpStream, MessagesCodec>, Message>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // send initial SYNC message
        debug!("Sending sync message");

        let sync_msg = self.create_sync_message();
        sink.send(sync_msg).await?;

        loop {
            delay_for(Duration::from_secs(5)).await;

            debug!("Sending update message");
            let upd_msg = self.create_update_message();
            sink.send(upd_msg).await?;
        }

        // Ok(())
    }

    fn create_sync_message(&self) -> Message {
        Message {
            client: Some(MsgClient {
                client_guid: self.sguid.clone(),
                name: self.name.clone(),
                position: self.pos.clone(),
                coalition: Coalition::Blue,
                radio_info: Some(RadioInfo {
                    name: "DATIS Radios".to_string(),
                    pos: self.pos.clone(),
                    ptt: false,
                    radios: vec![Radio {
                        enc: false,
                        enc_key: 0,
                        enc_mode: 0, // no encryption
                        freq_max: 1.0,
                        freq_min: 1.0,
                        freq: self.freq as f64,
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
                    unit: self
                        .unit
                        .as_ref()
                        .map(|u| u.name.clone())
                        .unwrap_or_else(|| self.name.clone()),
                    unit_id: self.unit.as_ref().map(|u| u.id).unwrap_or(0),
                    simultaneous_transmission: true,
                }),
            }),
            msg_type: MsgType::Sync,
            version: SRS_VERSION.to_string(),
        }
    }

    fn create_update_message(&self) -> Message {
        Message {
            client: Some(MsgClient {
                client_guid: self.sguid.clone(),
                name: self.name.clone(),
                position: self.pos.clone(),
                coalition: Coalition::Blue,
                radio_info: None,
            }),
            msg_type: MsgType::Update,
            version: SRS_VERSION.to_string(),
        }
    }
}
