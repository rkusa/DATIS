use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use crate::message::{create_sguid, Position};
use crate::voice_stream::VoiceStream;

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
    pos: Arc<RwLock<Position>>,
    unit: Option<UnitInfo>,
}

impl Client {
    pub fn new(name: &str, freq: u64) -> Self {
        Client {
            sguid: create_sguid(),
            name: name.to_string(),
            freq,
            pos: Arc::new(RwLock::new(Position::default())),
            unit: None,
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

    pub fn position(&self) -> Position {
        let p = self.pos.read().unwrap();
        p.clone()
    }

    pub fn unit(&self) -> Option<&UnitInfo> {
        self.unit.as_ref()
    }

    pub fn set_position(&mut self, pos: Position) {
        let mut p = self.pos.write().unwrap();
        *p = pos;
    }

    pub fn set_unit(&mut self, id: u32, name: &str) {
        self.unit = Some(UnitInfo {
            id,
            name: name.to_string(),
        });
    }

    pub async fn start(
        self,
        addr: SocketAddr,
        recv_voice: bool,
    ) -> Result<VoiceStream, anyhow::Error> {
        let stream = VoiceStream::new(self, addr, recv_voice).await?;
        Ok(stream)
    }
}
