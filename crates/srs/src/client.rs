use std::net::SocketAddr;
use std::sync::Arc;

use crate::message::{create_sguid, Coalition, GameMessage, LatLngPosition};
use crate::voice_stream::{VoiceStream, VoiceStreamError};

use futures::channel::mpsc;
use tokio::sync::oneshot::Receiver;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct UnitInfo {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Client {
    sguid: String,
    name: String,
    freq: u64,
    pos: Arc<RwLock<LatLngPosition>>,
    unit: Option<UnitInfo>,
    pub coalition: Coalition,
}

impl Client {
    pub fn new(name: &str, freq: u64, coalition: Coalition) -> Self {
        Client {
            sguid: create_sguid(),
            name: name.to_string(),
            freq,
            pos: Arc::new(RwLock::new(LatLngPosition::default())),
            unit: None,
            coalition,
        }
    }

    pub fn sguid(&self) -> &str {
        &self.sguid
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn freq(&self) -> u64 {
        self.freq
    }

    pub async fn position(&self) -> LatLngPosition {
        let p = self.pos.read().await;
        p.clone()
    }

    pub fn position_handle(&self) -> Arc<RwLock<LatLngPosition>> {
        self.pos.clone()
    }

    pub fn unit(&self) -> Option<&UnitInfo> {
        self.unit.as_ref()
    }

    pub async fn set_position(&mut self, pos: LatLngPosition) {
        let mut p = self.pos.write().await;
        *p = pos;
    }

    pub fn set_unit(&mut self, id: u32, name: &str) {
        self.unit = Some(UnitInfo {
            id,
            name: name.to_string(),
        });
    }

    /**
      Start sending updates to the specified server. If `game_source` is None,
      the client will act as a stationary transmitter using the position and
      frequency specified in the `Client` struct. It will not request any voice
      messages

      If the `game_source` is set, the position and frequencies of the game
      message will be sent, and voice requested
    */
    pub async fn start(
        self,
        addr: SocketAddr,
        game_source: Option<mpsc::UnboundedReceiver<GameMessage>>,
        shutdown_signal: Receiver<()>,
    ) -> Result<VoiceStream, VoiceStreamError> {
        let stream = VoiceStream::new(self, addr, game_source, shutdown_signal).await?;
        Ok(stream)
    }
}
