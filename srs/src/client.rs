use crate::message::{create_sguid, Position};
use crate::voice_stream::VoiceStream;
use tokio_net::ToSocketAddrs;

pub struct UnitInfo {
    pub id: u32,
    pub name: String,
}

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

    pub async fn start<A: ToSocketAddrs + Copy>(
        self,
        addr: A,
    ) -> Result<VoiceStream, anyhow::Error> {
        let stream = VoiceStream::new(self, addr).await?;
        Ok(stream)
    }
}
