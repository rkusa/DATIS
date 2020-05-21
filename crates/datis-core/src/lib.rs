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
pub mod rpc;
pub mod station;
pub mod tts;
mod utils;

use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::export::ReportExporter;
use crate::station::{LatLngPosition, Station, Transmitter};
use crate::tts::{
    aws::{self, AmazonWebServicesConfig},
    gcloud::{self, GoogleCloudConfig},
    win::{self, WindowsConfig},
    TextToSpeechConfig, TextToSpeechProvider,
};
use futures::future::FutureExt;
use futures::select;
use futures::sink::SinkExt;
use futures::stream::{SplitSink, StreamExt};
use srs::{Client, VoiceStream};
use tokio::runtime::{self, Runtime};
use tokio::sync::oneshot;
use tokio::time::delay_for;

pub struct Datis {
    stations: Vec<Station>,
    exporter: Option<ReportExporter>,
    gcloud_key: Option<String>,
    aws_config: Option<AwsConfig>,
    port: u16,
    runtime: Runtime,
    started: bool,
    shutdown_signals: Vec<oneshot::Sender<()>>,
    executable_path: Option<String>,
}

struct AwsConfig {
    key: String,
    secret: String,
    region: String,
}

impl Datis {
    pub fn new(stations: Vec<Station>) -> Result<Self, anyhow::Error> {
        Ok(Datis {
            stations,
            exporter: None,
            gcloud_key: None,
            aws_config: None,
            port: 5002,
            runtime: runtime::Builder::new()
                .threaded_scheduler()
                .enable_all()
                .build()?,
            started: false,
            shutdown_signals: Vec::new(),
            executable_path: None,
        })
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn set_gcloud_key<S: Into<String>>(&mut self, key: S) {
        self.gcloud_key = Some(key.into());
    }

    pub fn set_aws_keys<K: Into<String>, S: Into<String>, R: Into<String>>(
        &mut self,
        key: K,
        secret: S,
        region: R,
    ) {
        self.aws_config = Some(AwsConfig {
            key: key.into(),
            secret: secret.into(),
            region: region.into(),
        });
    }

    pub fn set_log_dir<S: Into<String>>(&mut self, log_dir: S) {
        let exporter = ReportExporter::new(log_dir.into() + "atis-reports.json");
        self.exporter = Some(exporter);
    }

    pub fn set_executable_path<S: Into<String>>(&mut self, executable_path: S) {
        self.executable_path = Some(executable_path.into());
    }

    pub fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.started {
            return Ok(());
        }

        self.started = true;

        for station in &mut self.stations {
            let config = match station.tts {
                TextToSpeechProvider::GoogleCloud { voice } => {
                    if let Some(ref key) = self.gcloud_key {
                        TextToSpeechConfig::GoogleCloud(GoogleCloudConfig {
                            key: key.clone(),
                            voice,
                        })
                    } else {
                        error!(
                            "Cannot start {} with TTS provider {:?} due to missing Google Cloud key",
                            station.name, station.tts
                        );
                        continue;
                    }
                }
                TextToSpeechProvider::AmazonWebServices { voice } => {
                    if let Some(AwsConfig {
                        ref key,
                        ref secret,
                        ref region,
                    }) = self.aws_config
                    {
                        TextToSpeechConfig::AmazonWebServices(AmazonWebServicesConfig {
                            key: key.clone(),
                            secret: secret.clone(),
                            region: match rusoto_core::Region::from_str(region) {
                                Ok(region) => region,
                                Err(err) => {
                                    error!(
                                        "Cannot start {} due to invalid AWS region {}: {}",
                                        station.name, region, err
                                    );
                                    continue;
                                }
                            },
                            voice,
                        })
                    } else {
                        error!(
                            "Cannot start {} due to missing AWS key, secret or region",
                            station.name
                        );
                        continue;
                    }
                }
                TextToSpeechProvider::Windows { ref voice } => {
                    TextToSpeechConfig::Windows(WindowsConfig {
                        executable_path: self.executable_path.clone(),
                        voice: voice.clone(),
                    })
                }
            };

            let (tx, rx) = oneshot::channel();
            self.shutdown_signals.push(tx);
            self.runtime.spawn(
                spawn(
                    station.clone(),
                    self.port,
                    config,
                    self.exporter.clone(),
                    rx,
                )
                .map(|_| ()),
            );
        }

        debug!("Started all ATIS stations");

        Ok(())
    }

    pub fn stop(mut self) -> Result<(), anyhow::Error> {
        self.pause()
    }

    pub fn resume(&mut self) -> Result<(), anyhow::Error> {
        self.start()
    }

    pub fn pause(&mut self) -> Result<(), anyhow::Error> {
        debug!("Shutting down all stations");

        let shutdown_signals = mem::replace(&mut self.shutdown_signals, Vec::new());
        for signal in shutdown_signals {
            let _ = signal.send(());
        }

        self.started = false;

        Ok(())
    }
}

async fn spawn(
    station: Station,
    port: u16,
    tts_config: TextToSpeechConfig,
    exporter: Option<ReportExporter>,
    shutdown_signal: oneshot::Receiver<()>,
) {
    let name = format!("ATIS {}", station.name);
    debug!("Connecting {} to 127.0.0.1:{}", name, port);

    let mut shutdown_signal = shutdown_signal.fuse();
    loop {
        let (tx, rx) = oneshot::channel();
        let mut r = Box::pin(run(&station, port, &tts_config, exporter.as_ref(), rx)).fuse();

        select! {
            result = r => {
                if let Err(err) = result
                {
                    error!("{} failed: {:?}", name, err);
                }

                info!("Restarting ATIS {} in 60 seconds ...", station.name);
                // TODO: handle shutdown signal during the delay
                delay_for(Duration::from_secs(60)).await;
            }
            _ = shutdown_signal => {
                let _ = tx.send(());
                let _ = r.await; // run until stopped
                break;
            }
        }
    }
}

async fn run(
    station: &Station,
    port: u16,
    tts_config: &TextToSpeechConfig,
    exporter: Option<&ReportExporter>,
    shutdown_signal: oneshot::Receiver<()>,
) -> Result<(), anyhow::Error> {
    let name = format!("ATIS {}", station.name);
    let mut client = Client::new(&name, station.freq);
    match &station.transmitter {
        Transmitter::Airfield(airfield) => {
            let pos = if let Some(rpc) = &station.rpc {
                rpc.to_lat_lng(&airfield.position).await?
            } else {
                LatLngPosition::default()
            };
            client.set_position(pos);
            // TODO: set unit?
        }
        Transmitter::Carrier(unit) => {
            client.set_unit(unit.unit_id, &unit.unit_name);
        }
        Transmitter::Custom(custom) => {
            client.set_unit(custom.unit_id, &custom.unit_name);
        }
        Transmitter::Weather(weather) => {
            client.set_unit(weather.unit_id, &weather.unit_name);
        }
    }
    let pos = client.position_handle();

    let (tx, rx) = oneshot::channel();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let (sink, stream) = client.start(addr, None, rx).await?.split();

    let mut stream = stream.fuse();
    let mut shutdown_signal = shutdown_signal.fuse();
    let mut broadcast = Box::pin(audio_broadcast(sink, station, pos, tts_config, exporter)).fuse();

    loop {
        select! {
            packet = stream.next() => {
                if let Some(packet) = packet {
                    packet?;
                    // we are currently not interested in the received voice packets, so simply discard them
                }
            }

            result = broadcast => {
                return result;
            }

            _ = shutdown_signal => {
                // shutdown socket
                let _ =tx.send(());

                break;
            }
        }
    }

    log::debug!("Station {} successfully shut down", station.name);

    Ok(())
}

async fn audio_broadcast(
    mut sink: SplitSink<VoiceStream, Vec<u8>>,
    station: &Station,
    position: Arc<RwLock<LatLngPosition>>,
    tts_config: &TextToSpeechConfig,
    exporter: Option<&ReportExporter>,
) -> Result<(), anyhow::Error> {
    let interval = match &station.transmitter {
        Transmitter::Weather(_) => {
            Duration::from_secs(60 * 15) // 15min
        }
        _ => {
            Duration::from_secs(60 * 60) // 60min
        }
    };
    let mut interval_start;
    let mut report_ix = 0;
    let mut previous_report = "".to_string();
    let mut frames = Vec::new();

    loop {
        interval_start = Instant::now();

        let report = match station.generate_report(report_ix).await? {
            Some(report) => report,
            None => {
                debug!(
                    "No report available for station {}. Trying again in 30 seconds ...",
                    station.name
                );
                // postpone the next playback of the report by some seconds ...
                delay_for(Duration::from_secs(30)).await;
                continue;
            }
        };
        if let Some(exporter) = exporter {
            if let Err(err) = exporter.export(&station.name, report.textual) {
                error!("Error exporting report: {}", err);
            }
        }

        debug!("{} Position: {:?}", station.name, report.position);

        {
            let mut pos = position.write().unwrap();
            *pos = report.position;
        }

        report_ix += 1;
        debug!("Report: {}", report.spoken);

        if report.spoken != previous_report {
            debug!("{} report has changed -> executing TTS", station.name);
            // only to TTS if the report has changed from the previous iteration
            frames = match tts_config {
                TextToSpeechConfig::GoogleCloud(config) => {
                    gcloud::text_to_speech(&report.spoken, config).await?
                }
                TextToSpeechConfig::AmazonWebServices(config) => {
                    aws::text_to_speech(&report.spoken, config).await?
                }
                TextToSpeechConfig::Windows(config) => {
                    win::text_to_speech(&report.spoken, config).await?
                }
            };
        }
        previous_report = report.spoken;

        loop {
            let elapsed = Instant::now() - interval_start;
            if elapsed > interval {
                // every 60min, generate a new report
                break;
            }

            let start = Instant::now();

            for (i, frame) in frames.iter().enumerate() {
                sink.send(frame.to_vec()).await?;

                // wait for the current ~playtime before sending the next package
                let playtime = Duration::from_millis((i as u64 + 1) * 20); // 20m per frame count
                let elapsed = start.elapsed();
                if playtime > elapsed {
                    delay_for(playtime - elapsed).await;
                }
            }

            // postpone the next playback of the report by some seconds ...
            match &station.transmitter {
                Transmitter::Airfield(_) | Transmitter::Weather(_) => {
                    delay_for(Duration::from_secs(3)).await;
                }
                Transmitter::Carrier(_) => {
                    delay_for(Duration::from_secs(10)).await;
                    // always create a new report for carriers, since they are usually
                    // constantly moving
                    break;
                }
                Transmitter::Custom(_) => {
                    delay_for(Duration::from_secs(1)).await;
                    // always create a new report to get an update on the position of the
                    // broadcasting unit
                    break;
                }
            }
        }
    }
}
