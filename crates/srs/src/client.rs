use std::net::SocketAddr;

use crate::message::{create_sguid, Position};
use crate::voice_stream::VoiceStream;

#[derive(Debug, Clone)]
pub struct UnitInfo {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub sguid: String,
    pub name: String,
    pub freq: u64,
    pub pos: Position,
    pub unit: Option<UnitInfo>,
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

    pub fn set_position(&mut self, pos: Position) {
        self.pos = pos;
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
