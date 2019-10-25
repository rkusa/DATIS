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

use std::future::Future;
use std::io::Cursor;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::time::{Duration, Instant};

use crate::export::ReportExporter;
use crate::station::Station;
use crate::tts::{
    aws::{self, AmazonWebServicesConfig},
    gcloud::{self, GoogleCloudConfig},
    TextToSpeechConfig, TextToSpeechProvider,
};
use audiopus::{coder::Encoder, Application, Channels, SampleRate};
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
    aws_config: Option<AwsConfig>,
    port: u16,
    runtime: Runtime,
    started: bool,
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
                            region: region.clone(),
                            voice,
                        })
                    } else {
                        error!("Cannot start {} with TTS provider {:?} due to missing AWS key, secret or region", station.name, station.tts);
                        continue;
                    }
                }
            };
            self.runtime.spawn(spawn(
                station.clone(),
                self.port,
                config,
                self.exporter.clone(),
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
    let tx: Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> = match tts_config {
        TextToSpeechConfig::GoogleCloud(config) => {
            Box::pin(audio_broadcast_gcloud(sink, station, config, exporter))
        }
        TextToSpeechConfig::AmazonWebServices(config) => {
            Box::pin(audio_broadcast_aws(sink, station, config, exporter))
        }
    };

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

async fn audio_broadcast_gcloud(
    mut sink: SplitSink<VoiceStream, Vec<u8>>,
    station: &Station,
    tts_config: &GoogleCloudConfig,
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

        let data = gcloud::text_to_speech(&report, tts_config).await?;
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

            let mut frame_count = 0;

            while let Some(pck) = audio.read_packet()? {
                let pck_size = pck.data.len();
                if pck_size == 0 {
                    continue;
                }
                size += pck_size;

                sink.send(pck.data).await?;

                // wait for the current ~playtime before sending the next package
                frame_count += 1;
                let playtime = Duration::from_millis(frame_count * 20); // 20m per frame count
                let elapsed = start.elapsed();
                if playtime > elapsed {
                    delay_for(playtime - elapsed).await;
                }
            }

            let playtime = Duration::from_millis(frame_count * 20); // 20m per frame count
            let elapsed = start.elapsed();
            debug!(
                "elapsed {:?}, {} frames, size {} bytes",
                elapsed, frame_count, size
            );

            if playtime > elapsed {
                delay_for(playtime - elapsed).await;
            }

            // postpone the next playback of the report by some seconds ...
            delay_for(Duration::from_secs(3)).await;

            data = audio.into_inner();
        }
    }
}

async fn audio_broadcast_aws(
    mut sink: SplitSink<VoiceStream, Vec<u8>>,
    station: &Station,
    tts_config: &AmazonWebServicesConfig,
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

        let data = aws::text_to_speech(&report, &tts_config).await?;
        let enc = Encoder::new(SampleRate::Hz16000, Channels::Mono, Application::Voip)?;

        loop {
            let elapsed = Instant::now() - interval_start;
            if elapsed > interval {
                // every 60min, generate a new report
                break;
            }

            let mut size = 0;
            const MONO_20MS: usize = 16000 * 1 * 20 / 1000;
            const SRS_DELAY_FACTOR: u64 = 40; //divide by 100 so it's actually 0.4
            let mut start_pos = 0;
            let mut end_pos = MONO_20MS;
            let mut output = [0; 256];
            let mut srs_out = true;

            while start_pos < data.len() {
                //cut off the last frame
                if end_pos > data.len() {
                    srs_out = false;
                    start_pos += MONO_20MS;
                    end_pos += MONO_20MS;
                }

                //play out to srs
                if srs_out {
                    // encode to opus
                    let bytes_written = enc.encode(&data[start_pos..end_pos], &mut output)?;
                    start_pos += MONO_20MS;
                    end_pos += MONO_20MS;

                    //pack frame
                    sink.send(output[..bytes_written].to_vec()).await?;

                    size = bytes_written;
                }

                if srs_out {
                    //Flexing the sleep time based on the size of the opus packet written
                    let this_delay = (size as f64 * SRS_DELAY_FACTOR as f64) / 100 as f64;
                    delay_for(Duration::from_micros(1000 * this_delay as u64)).await;
                }
            }

            //inserting 3000 ms break between broadcasts as we still cut off the transmission too early
            delay_for(Duration::from_millis(3000 as u64)).await;
        }
    }
}
