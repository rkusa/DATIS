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

use std::io::Cursor;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use crate::export::ReportExporter;
use crate::station::Station;
use crate::tts::text_to_speech;
use futures::future::{self, Either};
use futures_util::sink::SinkExt;
use futures_util::stream::{SplitSink, SplitStream, StreamExt};
use ogg::reading::PacketReader;
use srs::{Client, VoiceStream};
use tokio::runtime::Runtime;
use tokio::timer::delay_for;

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
            self.runtime.spawn(spawn(
                station.clone(),
                self.port,
                self.exporter.clone(),
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

async fn spawn(
    station: Station,
    port: u16,
    exporter: Option<ReportExporter>,
    gcloud_key: Option<String>,
) {
    let name = format!("ATIS {}", station.name);
    debug!("Connecting {} to 127.0.0.1:{}", name, port);

    loop {
        if let Err(err) = run(
            &station,
            port,
            exporter.as_ref(),
            gcloud_key.as_ref().map(|x| &**x),
        )
        .await
        {
            error!("{} failed: {:?}", name, err);
        }

        info!("Restarting ATIS {} in 10 seconds ...", station.name);
        delay_for(Duration::from_secs(10)).await;
    }
}

async fn run(
    station: &Station,
    port: u16,
    exporter: Option<&ReportExporter>,
    gcloud_key: Option<&str>,
) -> Result<(), anyhow::Error> {
    let name = format!("ATIS {}", station.name);
    let mut client = Client::new(&name, station.atis_freq);
    client.set_position(station.airfield.position.clone());
    // TODO: set unit

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let (sink, stream) = client.start(addr).await?.split();

    let rx = Box::pin(recv_voice_packets(stream));
    let tx = Box::pin(audio_broadcast(sink, station, exporter, gcloud_key));

    match future::try_select(rx, tx).await {
        Err(Either::Left((err, _))) => Err(err.into()),
        Err(Either::Right((err, _))) => Err(err.into()),
        _ => Ok(()),
    }
}

async fn recv_voice_packets(mut stream: SplitStream<VoiceStream>) -> Result<(), anyhow::Error> {
    while let Some(packet) = stream.next().await {
        packet?;
        // we are currently not interested in the received voice packets, so simply discard them
    }

    Ok(())
}

async fn audio_broadcast(
    mut sink: SplitSink<VoiceStream, Vec<u8>>,
    station: &Station,
    exporter: Option<&ReportExporter>,
    gcloud_key: Option<&str>,
) -> Result<bool, anyhow::Error> {
    let gcloud_key = gcloud_key.ok_or_else(|| anyhow!("Gcloud key is not set"))?;
    let interval = Duration::from_secs(60 * 60); // 60min
    let mut interval_start;
    let mut report_ix = 0;

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

        let data = text_to_speech(&gcloud_key, &report, station.voice).await?;
        let mut data = Cursor::new(data);

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

                sink.send(pck.data).await?;

                // wait for the current ~playtime before sending the next package
                let secs = (size * 8) as f64 / 1024.0 / 32.0; // 32 kBit/s
                let playtime = Duration::from_millis((secs * 1000.0) as u64);
                let elapsed = start.elapsed();
                if playtime > elapsed {
                    delay_for(playtime - elapsed).await;
                }
            }

            debug!("TOTAL SIZE: {}", size);

            // 32 kBit/s
            let secs = (size * 8) as f64 / 1024.0 / 32.0;
            debug!("SECONDS: {}", secs);

            let playtime = Duration::from_millis((secs * 1000.0) as u64);
            let elapsed = Instant::now() - start;
            if playtime > elapsed {
                delay_for(playtime - elapsed).await;
            }

            data = audio.into_inner();
        }
    }
}
