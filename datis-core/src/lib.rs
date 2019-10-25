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

use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::export::ReportExporter;
use crate::station::Station;
use crate::tts::{
    aws::{self, AmazonWebServicesConfig},
    gcloud::{self, GoogleCloudConfig},
    TextToSpeechConfig, TextToSpeechProvider,
};
use futures::future::{self, abortable, AbortHandle, Either, FutureExt};
use futures_util::sink::SinkExt;
use futures_util::stream::{SplitSink, SplitStream, StreamExt};
use srs::{Client, VoiceStream};
use tokio::runtime::Runtime;
use tokio::timer::delay_for;

pub struct Datis {
    stations: Vec<Station>,
    exporter: Option<ReportExporter>,
    gcloud_key: Option<String>,
    aws_config: Option<AwsConfig>,
    port: u16,
    runtime: Runtime,
    started: bool,
    abort_handles: Vec<AbortHandle>,
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
            runtime: Runtime::new()?,
            started: false,
            abort_handles: Vec::new(),
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
            };

            let (f, abort_handle) = abortable(spawn(
                station.clone(),
                self.port,
                config,
                self.exporter.clone(),
            ));
            self.abort_handles.push(abort_handle);
            self.runtime.spawn(f.map(|_| ()));
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
        let abort_handles = mem::replace(&mut self.abort_handles, Vec::new());
        for handle in abort_handles {
            handle.abort();
        }

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
    tts_config: TextToSpeechConfig,
    exporter: Option<ReportExporter>,
) {
    let name = format!("ATIS {}", station.name);
    debug!("Connecting {} to 127.0.0.1:{}", name, port);

    loop {
        if let Err(err) = run(&station, port, &tts_config, exporter.as_ref()).await {
            error!("{} failed: {:?}", name, err);
        }

        info!("Restarting ATIS {} in 10 seconds ...", station.name);
        delay_for(Duration::from_secs(10)).await;
    }
}

async fn run(
    station: &Station,
    port: u16,
    tts_config: &TextToSpeechConfig,
    exporter: Option<&ReportExporter>,
) -> Result<(), anyhow::Error> {
    let name = format!("ATIS {}", station.name);
    let mut client = Client::new(&name, station.atis_freq);
    client.set_position(station.airfield.position.clone());
    // TODO: set unit

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let (sink, stream) = client.start(addr).await?.split();

    let rx = Box::pin(recv_voice_packets(stream));
    let tx = Box::pin(audio_broadcast(sink, station, tts_config, exporter));

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
    tts_config: &TextToSpeechConfig,
    exporter: Option<&ReportExporter>,
) -> Result<(), anyhow::Error> {
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

        let frames = match tts_config {
            TextToSpeechConfig::GoogleCloud(config) => {
                gcloud::text_to_speech(&report, config).await?
            }
            TextToSpeechConfig::AmazonWebServices(config) => {
                aws::text_to_speech(&report, config).await?
            }
        };

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
            delay_for(Duration::from_secs(3)).await;
        }
    }
}
