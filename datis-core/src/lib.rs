#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate anyhow;

pub mod export;
pub mod station;
pub mod tts;
mod utils;
pub mod weather;

use std::io::{self, Cursor, Write};
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use std::{mem, thread};

use crate::export::ReportExporter;
use crate::station::Station;
use crate::tts::text_to_speech;
use byteorder::{LittleEndian, WriteBytesExt};
use ogg::reading::PacketReader;
use srs::Client;
use tokio::runtime::Runtime;
use tokio::timer::delay_for;

const MAX_FRAME_LENGTH: usize = 1024;

pub struct Datis {
    stations: Vec<Station>,
    exporter: Option<ReportExporter>,
    gcloud_key: Option<String>,
    port: u16,
    runtime: Runtime,
    started: bool,
}

impl Datis {
    pub fn new(stations: Vec<Station>) -> Result<Self, anyhow::Error> {
        Ok(Datis {
            stations,
            exporter: None,
            gcloud_key: None,
            port: 5002,
            runtime: Runtime::new()?,
            started: false,
        })
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn set_gcloud_key<S: Into<String>>(&mut self, key: S) {
        self.gcloud_key = Some(key.into());
    }

    pub fn set_log_dir<S: Into<String>>(&mut self, log_dir: S) {
        let exporter = ReportExporter::new(log_dir.into() + "atis-reports.json");
        self.exporter = Some(exporter);
    }

    pub fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.started {
            return Ok(());
        }

        self.started = true;

        for station in &mut self.stations {
            let name = format!("ATIS {}", station.name);
            self.runtime.spawn(spawn(
                name,
                station.atis_freq,
                self.port,
                self.gcloud_key.clone(),
            ));
        }

        Ok(())
    }

    pub fn stop(mut self) -> Result<(), anyhow::Error> {
        self.pause()
    }

    pub fn resume(&mut self) -> Result<(), anyhow::Error> {
        self.start()
    }

    pub fn pause(&mut self) -> Result<(), anyhow::Error> {
        let rt = mem::replace(&mut self.runtime, Runtime::new()?);
        rt.shutdown_now();
        debug!("Shut down all ATIS stations");

        self.started = false;

        Ok(())
    }
}

async fn spawn(name: String, freq: u64, port: u16, gcloud_key: Option<String>) {
    debug!("Connecting {} to 127.0.0.1:{}", name, port);

    loop {
        if let Err(err) = run(&name, freq, port, gcloud_key.as_ref().map(|x| &**x)).await {
            error!("ATIS {} failed: {}", name, err);
            info!("Restarting ATIS in 10 seconds ...");
            delay_for(Duration::from_secs(10)).await;
        }
    }
}

async fn run(
    name: &str,
    freq: u64,
    port: u16,
    gcloud_key: Option<&str>,
) -> Result<(), anyhow::Error> {
    let client = Client::new(name, freq);
    let stream = client.start(("127.0.0.1", port)).await?;
    Ok(())
}

#[allow(unused)]
fn audio_broadcast(
    sguid: &str,
    gloud_key: &str,
    station: &Station,
    exporter: Option<&ReportExporter>,
    srs_port: u16,
) -> Result<bool, anyhow::Error> {
    let interval = Duration::from_secs(60 * 60); // 60min
    let mut interval_start;
    let mut report_ix = 0;
    let mut packet_nr: u64 = 1;
    loop {
        interval_start = Instant::now();

        let report = station.generate_report(report_ix, true)?;
        let report_textual = station.generate_report(report_ix, false)?;
        if let Some(exporter) = exporter {
            if let Err(err) = exporter.export(&station.name, report_textual) {
                error!("Error exporting report: {}", err);
            }
        }

        report_ix += 1;
        debug!("Report: {}", report);

        let data = text_to_speech(&gloud_key, &report, station.voice)?;
        let mut data = Cursor::new(data);

        let srs_addr = ("127.0.0.1", srs_port);
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;

        // Excess bytes are discarded, when receiving messages longer than our buffer. Since we want to discard the whole
        // message anyway, a buffer with a length 1 is fine here.
        let mut sink = [0; 1];

        loop {
            let elapsed = Instant::now() - interval_start;
            if elapsed > interval {
                // every 60min, generate a new report
                break;
            }

            data.set_position(0);
            let start = Instant::now();
            let mut size = 0;
            let mut audio = PacketReader::new(data);
            while let Some(pck) = audio.read_packet()? {
                let pck_size = pck.data.len();
                if pck_size == 0 {
                    continue;
                }
                size += pck_size;
                let frame = pack_frame(&sguid, packet_nr, station.atis_freq, &pck.data)?;
                socket.send_to(&frame, srs_addr)?;
                packet_nr = packet_nr.wrapping_add(1);

                // read and discard pending datagrams
                match socket.recv_from(&mut sink) {
                    Err(err) => match err.kind() {
                        io::ErrorKind::TimedOut => {}
                        _ => {
                            return Err(err.into());
                        }
                    },
                    _ => {}
                }

                // wait for the current ~playtime before sending the next package
                let secs = (size * 8) as f64 / 1024.0 / 32.0; // 32 kBit/s
                let playtime = Duration::from_millis((secs * 1000.0) as u64);
                let elapsed = start.elapsed();
                if playtime > elapsed {
                    thread::sleep(playtime - elapsed);
                }

                // if ctx.should_stop() {
                //     return Ok(false);
                // }
            }

            debug!("TOTAL SIZE: {}", size);

            // 32 kBit/s
            let secs = (size * 8) as f64 / 1024.0 / 32.0;
            debug!("SECONDS: {}", secs);

            let playtime = Duration::from_millis((secs * 1000.0) as u64);
            let elapsed = Instant::now() - start;
            if playtime > elapsed {
                thread::sleep(playtime - elapsed);
            }

            // if ctx.should_stop_timeout(Duration::from_secs(3)) {
            //     return Ok(false);
            // }

            data = audio.into_inner();
        }
    }

    //    Ok(())
}

#[allow(unused)]
fn pack_frame(sguid: &str, id: u64, freq: u64, rd: &[u8]) -> Result<Vec<u8>, io::Error> {
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
