use std::cell::RefCell;
use std::io::{self, BufRead, BufReader, Cursor, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use std::{fmt, thread};

use byteorder::{LittleEndian, WriteBytesExt};
use crate::error::Error;
use crate::station::{AtisStation, FinalStation};
use crate::utils::create_lua_state;
use ogg::reading::PacketReader;
use uuid::Uuid;

const MAX_FRAME_LENGTH: usize = 1024;

pub fn start(cpath: String, station: AtisStation) -> Result<(), Error> {
    let airfield = station.airfield.as_ref().unwrap();
    let code = format!(
        r#"
        local Weather = require 'Weather'
        local position = {{
            x = {},
            y = {},
            z = {},
        }}

        getWeather = function()
            local wind = Weather.getGroundWindAtPoint({{ position = position }})
            local temp, pressure = Weather.getTemperatureAndPressureAtPoint({{
                position = position
            }})

            return {{
                windSpeed = wind.v,
                windDir = wind.a,
                temp = temp,
                pressure = pressure,
            }}
        end
    "#,
        airfield.position.x, airfield.position.alt, airfield.position.y,
    );
    debug!("Loading Lua: {}", code);

    let new_state = create_lua_state(&cpath, &code)?;
    let station = FinalStation {
        name: station.name,
        atis_freq: station.atis_freq,
        traffic_freq: station.traffic_freq,
        airfield: station.airfield.unwrap(),
        static_wind: station.static_wind,
        state: RefCell::new(new_state),
    };

    let mut stream = TcpStream::connect("127.0.0.1:5002")?;
    stream.set_nodelay(true)?;

    let sguid = Uuid::new_v4();
    let sguid = base64::encode_config(sguid.as_bytes(), base64::URL_SAFE_NO_PAD);
    assert_eq!(sguid.len(), 22);
    let name = station.name.clone();
    let position = Position {
        x: station.airfield.position.x,
        z: station.airfield.position.y,
        y: station.airfield.position.alt,
    };

    let sync_msg = Message {
        client: Some(Client {
            client_guid: &sguid,
            name: &name,
            position: position.clone(),
            coalition: Coalition::Blue,
        }),
        msg_type: MsgType::Sync,
        version: "1.5.3.5",
    };

    serde_json::to_writer(&stream, &sync_msg)?;
    stream.write_all(&['\n' as u8])?;

    let mut data = Vec::new();
    let mut stream = BufReader::new(stream);

    data.clear();

    let bytes_read = stream.read_until(b'\n', &mut data)?;
    if bytes_read == 0 {
        // TODO: ??
//            return Ok(());
    }

    {
        let sguid = sguid.clone();
        thread::spawn(move || {
            // TODO: unwrap
            audio_broadcast(sguid, &station).unwrap();
        });
    }

    let mut last_update = Instant::now();
    loop {
        data.clear();

        let bytes_read = stream.read_until(b'\n', &mut data)?;
        if bytes_read == 0 {
            // TODO: ??
//            return Ok(());
        }

        let elapsed = Instant::now() - last_update;
        if elapsed > Duration::from_secs(5) {
            // send update
            let mut stream = stream.get_mut();
            let upd_msg = Message {
                client: Some(Client {
                    client_guid: &sguid,
                    name: &name,
                    position: position.clone(),
                    coalition: Coalition::Blue,
                }),
                msg_type: MsgType::Update,
                version: "1.5.3.5",
            };

            serde_json::to_writer(&mut stream, &upd_msg).unwrap();
            stream.write_all(&['\n' as u8])?;

            last_update = Instant::now();
            debug!("SRS Update sent");
        }
    }

    return Ok(());
}

fn audio_broadcast(sguid: String, station: &FinalStation<'_>) -> Result<(), Error> {
    #[derive(Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct AudioConfig<'a> {
        audio_encoding: &'a str,
        sample_rate_hertz: u32,
        speaking_rate: f32,
    }

    #[derive(Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct Input<'a> {
        text: &'a str,
    }

    #[derive(Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct Voice<'a> {
        language_code: &'a str,
        name: &'a str,
    }

    #[derive(Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct TextToSpeechRequest<'a> {
        audio_config: AudioConfig<'a>,
        input: Input<'a>,
        voice: Voice<'a>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct TextToSpeechResponse {
        audio_content: String,
    }

    let interval = Duration::from_secs(60 * 20);
    let mut interval_start;
    loop {
        interval_start = Instant::now();

        // TODO: unwrap
        let report = station.generate_report()?;
        info!("Report: {}", report);

        let payload = TextToSpeechRequest {
            audio_config: AudioConfig {
                audio_encoding: "OGG_OPUS",
                sample_rate_hertz: 16_000,
                speaking_rate: 0.9,
            },
            input: Input { text: &report },
            voice: Voice {
                language_code: "en-US",
                name: "en-US-Standard-C",
            },
        };

        let key = "AIzaSyBB9rHqNGlclJTzz6bOA4hjjRmZBpdQ1Gg";
        let url = format!(
            "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
            key
        );
        let client = reqwest::Client::new();
        let mut res = client.post(&url).json(&payload).send()?;
        let data: TextToSpeechResponse = res.json()?;
        let data = base64::decode(&data.audio_content)?;
        let mut data = Cursor::new(data);

        let mut stream = TcpStream::connect("127.0.0.1:5003")?;
        stream.set_nodelay(true)?;

        loop {
            let elapsed = Instant::now() - interval_start;
            if elapsed > interval {
                // every 20min, generate a new report
                break;
            }

            data.set_position(0);
            let start = Instant::now();
            let mut size = 0;
            let mut audio = PacketReader::new(data);
            let mut id: u64 = 1;
            while let Some(pck) = audio.read_packet()? {
                size += pck.data.len();
                let frame = pack_frame(&sguid, id, station.atis_freq, &pck.data)?;
                stream.write(&frame)?;
                id += 1;
                thread::sleep(Duration::from_millis(20));
            }

            info!("TOTAL SIZE: {}", size);

            // 32 kBit/s
            let secs = (size * 8) as f64 / 1024.0 / 32.0;
            info!("SECONDS: {}", secs);

            let playtime = Duration::from_millis((secs * 1000.0) as u64);
            let elapsed = Instant::now() - start;
            if playtime > elapsed {
                thread::sleep(playtime - elapsed);
            }

            thread::sleep(Duration::from_secs(3));

            data = audio.into_inner();
        }
    }

    //    Ok(())
}

fn pack_frame(sguid: &str, id: u64, freq: u64, rd: &Vec<u8>) -> Result<Vec<u8>, io::Error> {
    let mut frame = Cursor::new(Vec::with_capacity(MAX_FRAME_LENGTH));

    // header segment will be written at the end
    frame.set_position(6);

    // - AUDIO SEGMENT
    let len_before = frame.position();
    // AudioPart1
    frame.write_all(&rd)?;
    let len_audio_part = frame.position() - len_before;

    // - FREQUENCY SEGMENT
    let len_before = frame.position();
    // Frequency
    frame.write_f64::<LittleEndian>(freq as f64)?;
    // Modulation
    //    AM = 0,
    //    FM = 1,
    //    INTERCOM = 2,
    //    DISABLED = 3
    frame.write_all(&[0])?;
    // Encryption
    //    NO_ENCRYPTION = 0,
    //    ENCRYPTION_JUST_OVERLAY = 1,
    //    ENCRYPTION_FULL = 2,
    //    ENCRYPTION_COCKPIT_TOGGLE_OVERLAY_CODE = 3
    frame.write_all(&[0])?;
    let len_frequency = frame.position() - len_before;

    // - FIXED SEGMENT
    // UnitId
    frame.write_u32::<LittleEndian>(0)?;
    // PacketId
    frame.write_u64::<LittleEndian>(id)?;
    // GUID
    frame.write_all(sguid.as_bytes())?;

    // - HEADER SEGMENT
    let len_packet = frame.get_ref().len();
    frame.set_position(0);
    // Packet Length
    frame.write_u16::<LittleEndian>(len_packet as u16)?;
    // AudioPart1 Length
    frame.write_u16::<LittleEndian>(len_audio_part as u16)?;
    // FrequencyPart Length
    frame.write_u16::<LittleEndian>(len_frequency as u16)?;

    Ok(frame.into_inner())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MsgType {
    Update,
    Sync,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Position {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Coalition {
    Blue,
    Red,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Client<'a> {
    client_guid: &'a str,
    name: &'a str,
    position: Position,
    coalition: Coalition,
    // RadioInfo
    // ClientChannelId
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Message<'a> {
    client: Option<Client<'a>>,
    msg_type: MsgType,
    // Clients
    // ServerSettings
    // ExternalAWACSModePassword
    version: &'a str,
}

impl ::serde::Serialize for MsgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        // Serialize the enum as a u64.
        serializer.serialize_u64(match *self {
            MsgType::Update => 1,
            MsgType::Sync => 2,
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
                    1 => Ok(MsgType::Update),
                    2 => Ok(MsgType::Sync),
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
            Coalition::Blue => 2,
            Coalition::Red => 1,
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