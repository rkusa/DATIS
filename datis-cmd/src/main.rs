#[macro_use]
extern crate log;

use std::str::FromStr;
use std::sync::Arc;

use clap::{App, Arg};
use datis_core::station::{Airfield, Position, Station};
use datis_core::tts::VoiceKind;
use datis_core::weather::StaticWeather;
use datis_core::Datis;
use dotenv::dotenv;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();

    let matches = App::new("dcs-radio-station")
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
            Arg::with_name("gcloud_key")
                .required(true)
                .long("gcloud")
                .env("GCLOUD_KEY"),
        )
        .get_matches();

    let freq = matches.value_of("frequency").unwrap();
    let freq = if let Ok(n) = u64::from_str(freq) {
        n
    } else {
        error!("The provided frequency is not a valid number");
        return Ok(());
    };
    // Calling .unwrap() is safe here because "gcloud_key" is required
    let gcloud_key = matches.value_of("gcloud_key").unwrap();

    let station = Station {
        name: String::from("Test Station"),
        atis_freq: freq,
        traffic_freq: None,
        voice: VoiceKind::StandardB,
        airfield: Airfield {
            name: String::from("Test"),
            position: Position::default(),
            runways: vec![String::from("09"), String::from("26")],
        },
        weather: Arc::new(StaticWeather),
    };
    let mut datis = Datis::new(vec![station])?;
    datis.set_port(5002);
    datis.set_gcloud_key(gcloud_key);
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
