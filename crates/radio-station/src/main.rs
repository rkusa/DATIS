#![warn(rust_2018_idioms)]

mod radio_station;

use std::str::FromStr;

use radio_station::RadioStation;

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();

    let matches = clap::App::new("dcs-radio-station")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            clap::Arg::with_name("frequency")
                .short("f")
                .long("freq")
                .default_value("251000000")
                .help("Sets the SRS frequency (in Hz, e.g. 255000000 for 255MHz)")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("port")
                .short("p")
                .long("port")
                .default_value("5002")
                .help("Sets the SRS Port")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("loop")
                .short("l")
                .long("loop")
                .help("Enables endlessly looping the audio file(s)"),
        )
        .arg(
            clap::Arg::with_name("PATH")
                .help("Sets the path audio file(s) should be read from")
                .required(true)
                .index(1),
        )
        .get_matches();

    // Calling .unwrap() is safe here because "INPUT" is required
    let path = matches.value_of("PATH").unwrap();
    let should_loop = matches.is_present("loop");
    let port = matches.value_of("port").unwrap();
    let port = if let Ok(n) = u16::from_str(port) {
        n
    } else {
        log::error!("The provided Port is not a valid number");
        return Ok(());
    };
    let freq = matches.value_of("frequency").unwrap();
    let freq = if let Ok(n) = u64::from_str(freq) {
        n
    } else {
        log::error!("The provided frequency is not a valid number");
        return Ok(());
    };

    let mut station = RadioStation::new("DCS Radio Station");
    station.set_frequency(freq);
    station.set_position(0.0, 0.0, 8000.);
    station.set_port(port);

    log::info!("Start playing ...");
    station.play(path, should_loop).await?;

    Ok(())
}
