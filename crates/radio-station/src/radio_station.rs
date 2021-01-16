use std::fs::File as SyncFile;
use std::io::Cursor;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use async_std::fs::File;
use async_std::io::prelude::ReadExt;
use async_std::path::{Path, PathBuf};
use futures::future::{self, Either};
use futures::sink::SinkExt;
use futures::stream::{SplitSink, SplitStream, StreamExt as FutStreamExt};
use ogg::reading::PacketReader;
use ogg_metadata::{AudioMetadata, OggFormat};
use srs::message::{Coalition, LatLngPosition};
use srs::{Client, VoiceStream};
use tokio::sync::oneshot;
use tokio::time::delay_for;

pub struct RadioStation {
    name: String,
    position: LatLngPosition,
    freq: u64,
    port: u16,
}

impl RadioStation {
    pub fn new(name: &str) -> Self {
        RadioStation {
            name: name.to_string(),
            position: LatLngPosition::default(),
            freq: 251_000_000,
            port: 5002,
        }
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn set_position(&mut self, lat: f64, lng: f64, alt: f64) {
        self.position = LatLngPosition { lat, lng, alt };
    }

    pub fn set_frequency(&mut self, freq: u64) {
        self.freq = freq;
    }

    pub async fn play<P: AsRef<Path>>(
        self,
        path: P,
        should_loop: bool,
    ) -> Result<(), anyhow::Error> {
        let mut client = Client::new(&self.name, self.freq, Coalition::Blue);
        client.set_position(self.position).await;

        let (_tx, rx) = oneshot::channel();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), self.port);
        let (sink, stream) = client.start(addr, None, rx).await?.split();

        let rx = Box::pin(recv_voice_packets(stream));
        let tx = Box::pin(radio_broadcast(sink, path, should_loop));

        match future::try_select(rx, tx).await {
            Err(Either::Left((err, _))) => Err(err),
            Err(Either::Right((err, _))) => Err(err),
            _ => Ok(()),
        }
    }
}

async fn recv_voice_packets(mut stream: SplitStream<VoiceStream>) -> Result<(), anyhow::Error> {
    while let Some(packet) = stream.next().await {
        packet?;
        // we are currently not interested in the received voice packets, so simply discard them
    }

    Ok(())
}

struct OpusFile {
    path: PathBuf,
    #[allow(unused)]
    duration: Duration,
}

async fn radio_broadcast<P: AsRef<Path>>(
    mut sink: SplitSink<VoiceStream, Vec<u8>>,
    path: P,
    should_loop: bool,
) -> Result<(), anyhow::Error> {
    let mut file_paths: Vec<PathBuf> = Vec::new();

    if path.as_ref().is_dir().await {
        let mut dir = path.as_ref().read_dir().await?;

        while let Some(entry) = dir.next().await {
            file_paths.push(entry?.path());
        }
    } else {
        file_paths.push(path.as_ref().to_path_buf())
    }

    let mut audio_files = Vec::new();
    for path in file_paths {
        if path.extension().is_none() || path.extension().unwrap() != "ogg" {
            log::warn!("Ignoring non .ogg file: {:?}", path);
            continue;
        }

        // FIXME: find an async way of reading the metadata
        let mut f = SyncFile::open(&path)?;
        if let Some(OggFormat::Opus(meta)) = ogg_metadata::read_format(&mut f)?.into_iter().next() {
            if let Some(duration) = meta.get_duration() {
                audio_files.push(OpusFile { path, duration });
            } else {
                log::error!("Failed reading duration of {}", path.to_string_lossy());
            }
        } else {
            log::error!("{} is not opus encoded", path.to_string_lossy());
        }
    }

    loop {
        for OpusFile { ref path, .. } in &audio_files {
            log::debug!("Playing {}", path.to_string_lossy());

            let mut file = File::open(&path).await?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).await?;

            let start = Instant::now();
            let mut audio = PacketReader::new(Cursor::new(contents));
            let mut frame_count = 0;

            while let Some(pck) = audio.read_packet()? {
                if pck.data.is_empty() {
                    continue;
                }

                sink.send(pck.data).await?;
                frame_count += 1;

                // wait for the current ~playtime before sending the next package
                let playtime = Duration::from_millis((frame_count as u64 + 1) * 20); // 20m per frame count
                let elapsed = start.elapsed();
                if playtime > elapsed {
                    delay_for(playtime - elapsed).await;
                }
            }
        }

        if !should_loop {
            break;
        }
    }

    Ok(())
}
