use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MsgType {
    Update,
    Ping,
    Sync,
    RadioUpdate,
    ServerSettings,
    ClientDisconnect,
    VersionMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Coalition {
    Spectator,
    Blue,
    Red,
}

#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    #[serde(rename = "z")]
    pub y: f64,
    #[serde(rename = "y")]
    pub alt: f64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Radio {
    pub enc: bool,
    pub enc_key: u8,
    pub enc_mode: u8,
    pub freq_max: f64,   // 1.0,
    pub freq_min: f64,   // 1.0,
    pub freq: f64,       // 1.0,
    pub modulation: u8,  // 3,
    pub name: String,    // "No Radio",
    pub sec_freq: f64,   // 0.0,
    pub volume: f32,     // 1.0,
    pub freq_mode: u8,   // 0,
    pub vol_mode: u8,    // 0,
    pub expansion: bool, // false,
    pub channel: i32,    // -1,
    pub simul: bool,     // false
}

impl From<&GameRadio> for Radio {
    fn from(r: &GameRadio) -> Self {
        Self {
            enc: r.enc,
            enc_key: r.enc_key,
            enc_mode: r.enc_mode,
            freq_max: r.freq_max,
            freq_min: r.freq_min,
            freq: r.freq,
            modulation: r.modulation,
            name: r.name.clone(),
            sec_freq: r.sec_freq,
            volume: r.volume,
            freq_mode: r.freq_mode,
            vol_mode: r.vol_mode,
            expansion: r.expansion,
            channel: -1,
            simul: false,
        }
    }

}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RadioInfo {
    pub name: String,
    pub pos: Position,
    pub ptt: bool,
    pub radios: Vec<Radio>,
    pub control: u8,
    pub selected: i16,
    pub unit: String,
    pub unit_id: u32,
    pub simultaneous_transmission: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Client {
    pub client_guid: String,
    pub name: Option<String>,
    pub position: Position,
    pub coalition: Coalition,
    pub radio_info: Option<RadioInfo>,
    // ClientChannelId
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    pub client: Option<Client>,
    pub msg_type: MsgType,
    // Clients
    // ServerSettings
    // ExternalAWACSModePassword
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameRadio {
    pub enc: bool,
    pub enc_key: u8,
    pub enc_mode: u8,
    pub freq_max: f64,   // 1.0,
    pub freq_min: f64,   // 1.0,
    pub freq: f64,       // 1.0,
    pub modulation: u8,  // 3,
    pub name: String,    // "No Radio",
    pub sec_freq: f64,   // 0.0,
    pub volume: f32,     // 1.0,
    pub freq_mode: u8,   // 0,
    pub vol_mode: u8,    // 0,
    pub expansion: bool, // false,
    pub guard_freq_mode: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameMessage {
    pub control: i32,
    pub name: String,
    pub pos: Position,
    pub ptt: bool,
    pub radios: Vec<GameRadio>,
    pub selected: i16,
    pub unit: String,
    pub unit_id: u32,
}

impl ::serde::Serialize for MsgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        // Serialize the enum as a u64.
        serializer.serialize_u64(match *self {
            MsgType::Update => 0,
            MsgType::Ping => 1,
            MsgType::Sync => 2,
            MsgType::RadioUpdate => 3,
            MsgType::ServerSettings => 4,
            MsgType::ClientDisconnect => 5,
            MsgType::VersionMismatch => 6,
        })
    }
}

impl<'de> ::serde::Deserialize<'de> for MsgType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = MsgType;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("positive integer")
            }

            fn visit_u64<E>(self, value: u64) -> Result<MsgType, E>
            where
                E: ::serde::de::Error,
            {
                // Rust does not come with a simple way of converting a
                // number to an enum, so use a big `match`.
                match value {
                    0 => Ok(MsgType::Update),
                    1 => Ok(MsgType::Ping),
                    2 => Ok(MsgType::Sync),
                    3 => Ok(MsgType::RadioUpdate),
                    4 => Ok(MsgType::ServerSettings),
                    5 => Ok(MsgType::ClientDisconnect),
                    6 => Ok(MsgType::VersionMismatch),
                    _ => Err(E::custom(format!(
                        "unknown {} value: {}",
                        stringify!(MsgType),
                        value
                    ))),
                }
            }
        }

        // Deserialize the enum from a u64.
        deserializer.deserialize_u64(Visitor)
    }
}

impl ::serde::Serialize for Coalition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        // Serialize the enum as a u64.
        serializer.serialize_u64(match *self {
            Coalition::Spectator => 0,
            Coalition::Red => 1,
            Coalition::Blue => 2,
        })
    }
}

impl<'de> ::serde::Deserialize<'de> for Coalition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Coalition;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("positive integer")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Coalition, E>
            where
                E: ::serde::de::Error,
            {
                // Rust does not come with a simple way of converting a
                // number to an enum, so use a big `match`.
                match value {
                    0 => Ok(Coalition::Spectator),
                    1 => Ok(Coalition::Red),
                    2 => Ok(Coalition::Blue),
                    _ => Err(E::custom(format!(
                        "unknown {} value: {}",
                        stringify!(Coalition),
                        value
                    ))),
                }
            }
        }

        // Deserialize the enum from a u64.
        deserializer.deserialize_u64(Visitor)
    }
}

pub fn create_sguid() -> String {
    let sguid = Uuid::new_v4();
    let sguid = base64::encode_config(sguid.as_bytes(), base64::URL_SAFE_NO_PAD);
    assert_eq!(sguid.len(), 22);
    sguid
}
