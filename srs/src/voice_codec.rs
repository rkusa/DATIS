use std::io::{self, Cursor, Read, Write};

// use byteorder::ByteOrder;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::BytesMut;
use tokio_codec::{Decoder, Encoder, LengthDelimitedCodec};

pub struct AudioCodec {
    inner: LengthDelimitedCodec,
}

impl AudioCodec {
    pub fn new() -> Self {
        AudioCodec {
            inner: LengthDelimitedCodec::builder()
                .length_field_offset(0)
                .length_field_length(2)
                .length_adjustment(0)
                .little_endian()
                .new_codec(),
        }
    }
}

pub enum Modulation {
    AM,
    FM,
    Intercom,
    Disabled,
}

pub enum Encryption {
    None,
    JustOverlay,
    Full,
    CockpitToggleOverlayCode,
}

pub struct Frequency {
    pub freq: f64,
    pub modulation: Modulation,
    pub encryption: Encryption,
}

pub struct VoicePacket {
    // TODO: use Bytes instead?
    pub audio_part: Vec<u8>,
    pub frequencies: Vec<Frequency>,
    pub unit_id: u32,
    pub packet_id: u64,
    pub sguid: [u8; 22],
}

impl Decoder for AudioCodec {
    type Item = VoicePacket;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(bytes) = self.inner.decode(buf)? {
            let len = buf.len() as u64;

            let mut rd = Cursor::new(bytes);
            let len_audio_part = rd.read_u16::<LittleEndian>()? as usize;
            let len_frequencies = rd.read_u16::<LittleEndian>()? as u64;

            let mut audio_part = Vec::with_capacity(len_audio_part);
            rd.read_exact(&mut audio_part)?;

            let len_before = rd.position();
            let mut frequencies = Vec::new();
            while len_before - rd.position() < len_frequencies {
                let freq = rd.read_f64::<LittleEndian>()?;
                let modulation = match rd.read_u8()? {
                    0 => Modulation::AM,
                    1 => Modulation::FM,
                    2 => Modulation::Intercom,
                    3 => Modulation::Disabled,
                    _ => Modulation::AM,
                };
                let encryption = match rd.read_u8()? {
                    0 => Encryption::None,
                    1 => Encryption::JustOverlay,
                    2 => Encryption::Full,
                    3 => Encryption::CockpitToggleOverlayCode,
                    _ => Encryption::None,
                };
                frequencies.push(Frequency {
                    freq,
                    modulation,
                    encryption,
                });
            }

            let unit_id = rd.read_u32::<LittleEndian>()?;
            let packet_id = rd.read_u64::<LittleEndian>()?;

            let mut sguid = [0; 22];
            rd.read_exact(&mut sguid)?;

            assert_eq!(rd.position(), len);

            Ok(Some(VoicePacket {
                audio_part,
                frequencies,
                unit_id,
                packet_id,
                sguid,
            }))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for AudioCodec {
    type Item = VoicePacket;
    type Error = io::Error;

    fn encode(&mut self, packet: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let capacity = 4 + packet.audio_part.len() + packet.frequencies.len() * 10 + 4 + 8 + 22;
        let mut wd = Cursor::new(Vec::with_capacity(capacity));

        // header segment will be written at the end
        wd.set_position(4);

        // - AUDIO SEGMENT
        let len_before = wd.position();
        wd.write_all(&packet.audio_part)?;
        let len_audio_part = wd.position() - len_before;

        // - FREQUENCY SEGMENT
        let len_before = wd.position();
        for f in packet.frequencies {
            wd.write_f64::<LittleEndian>(f.freq)?;

            wd.write_u8(match f.modulation {
                Modulation::AM => 0,
                Modulation::FM => 1,
                Modulation::Intercom => 2,
                Modulation::Disabled => 3,
            })?;
            wd.write_u8(match f.encryption {
                Encryption::None => 0,
                Encryption::JustOverlay => 1,
                Encryption::Full => 2,
                Encryption::CockpitToggleOverlayCode => 3,
            })?;
        }

        let len_frequency = wd.position() - len_before;

        // - FIXED SEGMENT
        wd.write_u32::<LittleEndian>(packet.unit_id)?;
        wd.write_u64::<LittleEndian>(packet.packet_id)?;
        wd.write_all(&packet.sguid)?;

        // - HEADER SEGMENT
        wd.set_position(0);

        // Packet Length:
        // the final packet will start with the total packet length, but this will be added by
        // the inner fixed codec

        // AudioPart1 Length
        wd.write_u16::<LittleEndian>(len_audio_part as u16)?;
        // FrequencyPart Length
        wd.write_u16::<LittleEndian>(len_frequency as u16)?;

        let frame = wd.into_inner();
        assert_eq!(frame.len(), capacity);

        self.inner.encode(frame.into(), buf)
    }
}
