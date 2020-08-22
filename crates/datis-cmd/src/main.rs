use std::str::FromStr;

use clap::{App, Arg};
use datis_core::station::{Airfield, Position, Station, Transmitter};
use datis_core::tts::TextToSpeechProvider;
use datis_core::Datis;
use dotenv::dotenv;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .unwrap();

    let matches = App::new("datis-cmd")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("frequency")
                .short("f")
                .long("freq")
                .default_value("251000000")
                .help("Sets the SRS frequency (in Hz, e.g. 251000000 for 251MHz)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("tts")
                .required(true)
                .long("tts")
                .default_value("GC:en-US-Standard-C")
                .help("Sets the TTS provider and voice to be used")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("gcloud_key")
                .long("gcloud")
                .env("GCLOUD_KEY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("aws_key")
                .long("aws-key")
                .env("AWS_ACCESS_KEY_ID")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("aws_secret")
                .long("aws-secret")
                .env("AWS_SECRET_ACCESS_KEY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("aws_region")
                .long("aws-region")
                .env("AWS_REGION")
                .default_value("EuCentral1")
                .takes_value(true),
        )
        .get_matches();

    let freq = matches.value_of("frequency").unwrap();
    let freq = if let Ok(n) = u64::from_str(freq) {
        n
    } else {
        log::error!("The provided frequency is not a valid number");
        return Ok(());
    };

    let tts = matches.value_of("tts").unwrap();
    let tts = match TextToSpeechProvider::from_str(&tts) {
        Ok(tts) => tts,
        Err(err) => {
            log::error!("The privided TTS provider/voice is invalid: {}", err);
            return Ok(());
        }
    };

    let station = Station {
        name: String::from("Test Station"),
        freq,
        tts: tts,
        transmitter: Transmitter::Airfield(Airfield {
            name: String::from("Test"),
            position: Position::default(),
            runways: vec![String::from("09"), String::from("26")],
            traffic_freq: None,
            info_ltr_offset: 0,
        }),
        rpc: None,
    };
    let mut datis = Datis::new(vec![station])?;
    datis.set_port(5002);

    if let Some(key) = matches.value_of("gcloud_key") {
        datis.set_gcloud_key(key);
    }

    if let (Some(key), Some(secret), Some(region)) = (
        matches.value_of("aws_key"),
        matches.value_of("aws_secret"),
        matches.value_of("aws_region"),
    ) {
        datis.set_aws_keys(key, secret, region);
    }

    datis.start()?;

    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    rx.recv().unwrap();
    datis.stop()?;

    Ok(())
}
