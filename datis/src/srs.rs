use std::io::{self, BufRead, BufReader, Cursor, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use std::{fmt, thread};

use byteorder::{LittleEndian, WriteBytesExt};
use crate::station::FinalStation;
use ogg::reading::PacketReader;
use uuid::Uuid;

const MAX_FRAME_LENGTH: usize = 1024;

pub fn start(station: FinalStation<'_>) -> Result<(), io::Error> {
    let mut stream = TcpStream::connect("127.0.0.1:5002")?;
    stream.set_nodelay(true)?;

    let sguid = Uuid::new_v4();
    let sguid = base64::encode_config(sguid.as_bytes(), base64::URL_SAFE_NO_PAD);
    assert_eq!(sguid.len(), 22);

    let airfield = station.airfield.as_ref().unwrap();
    let sync_msg = Message {
        client: Some(Client {
            client_guid: &sguid,
            name: &station.name,
            position: Position {
                x: airfield.position.x,
                y: airfield.position.alt,
                z: airfield.position.y,
            },
        }),
        msg_type: MsgType::Sync,
        version: "1.5.3.5",
    };

    serde_json::to_writer(&stream, &sync_msg).unwrap();
    stream.write_all(&['\n' as u8])?;

    let mut data = Vec::new();
    let mut stream = BufReader::new(stream);

    loop {
        data.clear();

        let bytes_read = stream.read_until(b'\n', &mut data)?;
        if bytes_read == 0 {
            return Ok(());
        }

        //        println!("RECEIVED: {}", String::from_utf8_lossy(&data));
        //        let msg: Message = serde_json::from_slice(&data).unwrap();

        //        thread::spawn(move || {
        audio_broadcast(sguid.clone(), &station)?;
        //        });

        break;
    }

    return Ok(());
}

fn audio_broadcast(sguid: String, station: &FinalStation<'_>) -> Result<(), io::Error> {
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

    // TODO: unwrap
    let report = match station.generate_report() {
        Ok(report) => report,
        Err(err) => {
            error!("{}", err);
            return Ok(());
        }
    };
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
    let mut res = client.post(&url).json(&payload).send().unwrap();
    let data: TextToSpeechResponse = res.json().unwrap();
    let data = base64::decode(&data.audio_content).unwrap();
    let mut data = Cursor::new(data);

    let mut stream = TcpStream::connect("127.0.0.1:5003")?;
    stream.set_nodelay(true)?;

    loop {
        data.set_position(0);
        let start = Instant::now();
        let mut size = 0;
        let mut audio = PacketReader::new(data);
        let mut id: u64 = 1;
        while let Some(pck) = audio.read_packet().unwrap() {
            size += pck.data.len();
            let frame = pack_frame(&sguid, id, station.freq, &pck.data)?;
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
    Sync,
}

#[derive(Serialize, Deserialize, Debug)]
struct Position {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Client<'a> {
    client_guid: &'a str,
    name: &'a str,
    position: Position,
    // Coalition
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
