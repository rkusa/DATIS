use std::sync::Arc;

use crate::tts::TextToSpeechProvider;
use crate::utils::{pronounce_number, round};
use crate::weather::{Clouds, Weather, WeatherInfo};
use anyhow::Context;
pub use srs::message::Position;

#[derive(Clone)]
pub struct Station {
    pub name: String,
    pub freq: u64,
    pub tts: TextToSpeechProvider,
    pub weather: Arc<dyn Weather + Send + Sync>,
    pub transmitter: Transmitter,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Airfield {
    pub name: String,
    pub position: Position,
    pub runways: Vec<String>,
    pub traffic_freq: Option<u64>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Carrier {
    pub name: String,
    pub unit_id: u32,
    pub unit_name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Transmitter {
    Airfield(Airfield),
    Carrier(Carrier),
}

pub struct Report {
    pub textual: String,
    pub spoken: String,
    pub position: Position,
}

impl Station {
    pub fn generate_report(&self, report_nr: usize) -> Result<Option<Report>, anyhow::Error> {
        match &self.transmitter {
            Transmitter::Airfield(airfield) => {
                let weather = self
                    .weather
                    .get_at(
                        airfield.position.x,
                        airfield.position.y,
                        airfield.position.alt,
                    )
                    .context("failed to retrieve weather")?;

                Ok(Some(Report {
                    textual: airfield.generate_report(report_nr, &weather, false)?,
                    spoken: airfield.generate_report(report_nr, &weather, true)?,
                    position: weather.position,
                }))
            }
            Transmitter::Carrier(unit) => {
                let weather = self
                    .weather
                    .get_for_unit(&unit.unit_name)
                    .context("failed to retrieve weather for unit")?;
                if let Some(weather) = weather {
                    Ok(Some(Report {
                        textual: unit.generate_report(&weather, false)?,
                        spoken: unit.generate_report(&weather, true)?,
                        position: weather.position,
                    }))
                } else {
                    return Ok(None);
                }
            }
        }
    }
}

impl Airfield {
    fn get_active_runway(&self, wind_dir: f64) -> Option<&str> {
        let lr: &[_] = &['L', 'R'];
        for rwy in &self.runways {
            let rwy = rwy.trim_matches(lr);
            if let Ok(mut rwy_dir) = rwy.parse::<f64>() {
                rwy_dir *= 10.0; // e.g. 04 to 040
                let phi = (wind_dir - rwy_dir).abs() % 360.0;
                let distance = if phi > 180.0 { 360.0 - phi } else { phi };
                if distance <= 90.0 {
                    return Some(&rwy);
                }
            } else {
                error!("Error parsing runway: {}", rwy);
            }
        }

        None
    }

    pub fn generate_report(
        &self,
        report_nr: usize,
        weather: &WeatherInfo,
        spoken: bool,
    ) -> Result<String, anyhow::Error> {
        #[cfg(not(test))]
        let _break = if spoken { "\n" } else { "" };
        #[cfg(test)]
        let _break = if spoken { "| " } else { "" };

        let information_letter = PHONETIC_ALPHABET[report_nr % PHONETIC_ALPHABET.len()];
        let mut report = if spoken { "<speak>\n" } else { "" }.to_string();

        report += &format!(
            "This is {} information {}. {}",
            self.name, information_letter, _break
        );

        if let Some(rwy) = self.get_active_runway(weather.wind_dir) {
            let rwy = pronounce_number(rwy, spoken);
            report += &format!("Runway in use is {}. {}", rwy, _break);
        } else {
            error!("Could not find active runway for {}", self.name);
        }

        let wind_dir = format!("{:0>3}", weather.wind_dir.round().to_string());
        report += &format!(
            "Wind {} at {} knots. {}",
            pronounce_number(wind_dir, spoken),
            pronounce_number(weather.wind_speed.round(), spoken),
            _break,
        );

        if let Some(visibility) = weather.visibility {
            report += &format!("{}. {}", get_visibility_report(visibility, spoken), _break);
        }

        if let Some(clouds_report) = weather
            .clouds
            .as_ref()
            .and_then(|clouds| get_clouds_report(clouds, spoken))
        {
            report += &format!("{}. {}", clouds_report, _break);
        }

        report += &format!(
            "Temperature {} celcius. {}",
            pronounce_number(round(weather.temperature, 1), spoken),
            _break,
        );

        report += &format!(
            "ALTIMETER {}. {}",
            // inHg, but using 0.02953 instead of 0.0002953 since we don't want to speak the
            // DECIMAL here
            pronounce_number((weather.pressure_qnh * 0.02953).round(), spoken),
            _break,
        );

        if let Some(traffic_freq) = self.traffic_freq {
            report += &format!(
                "Traffic frequency {}. {}",
                pronounce_number(round(traffic_freq as f64 / 1_000_000.0, 3), spoken),
                _break
            );
        }

        report += &format!("REMARKS. {}", _break,);
        report += &format!(
            "{} hectopascal. {}",
            pronounce_number((weather.pressure_qnh / 100.0).round(), spoken), // to hPA
            _break,
        );
        report += &format!(
            "QFE {} or {}. {}",
            pronounce_number((weather.pressure_qfe * 0.02953).round(), spoken), // to inHg
            pronounce_number((weather.pressure_qfe / 100.0).round(), spoken),   // to hPA
            _break,
        );

        report += &format!("End information {}.", information_letter);

        if spoken {
            report += "\n</speak>";
        }

        Ok(report)
    }
}

impl Carrier {
    pub fn generate_report(
        &self,
        weather: &WeatherInfo,
        spoken: bool,
    ) -> Result<String, anyhow::Error> {
        #[cfg(not(test))]
        let _break = if spoken { "\n" } else { "" };
        #[cfg(test)]
        let _break = if spoken { "| " } else { "" };

        let mut report = if spoken { "<speak>\n" } else { "" }.to_string();

        report += &format!("99, {}", _break);

        let wind_dir = format!("{:0>3}", weather.wind_dir.round().to_string());
        report += &format!(
            "{}'s wind {} at {} knots, {}",
            self.name,
            pronounce_number(wind_dir, spoken),
            pronounce_number(weather.wind_speed.round(), spoken),
            _break,
        );

        report += &format!(
            "altimeter {}, {}",
            // inHg, but using 0.02953 instead of 0.0002953 since we don't want to speak the
            // DECIMAL here
            pronounce_number((weather.pressure_qnh * 0.02953).round(), spoken),
            _break,
        );

        // TODO: determine CASE 1, 2, 3
        report += &format!("CASE 1, {}", _break,);

        // TODO: BRC xxx
        // TODO: expected final heading xxx
        // TODO: report initial

        if spoken {
            report += "\n</speak>";
        }

        Ok(report)
    }
}

fn get_visibility_report(visibility: u32, spoken: bool) -> String {
    // convert m to nm
    let visibility = (f64::from(visibility) * 0.000_539_957).round();
    format!("Visibility {}", pronounce_number(visibility, spoken))
}

fn get_clouds_report(clouds: &Clouds, spoken: bool) -> Option<String> {
    let density = match clouds.density {
        2..=5 => Some("few"),
        6..=7 => Some("scattered"),
        8 => Some("broken"),
        9..=10 => Some("overcast"),
        _ => None,
    };
    if let Some(density) = density {
        let mut report = String::new();
        // convert m to ft, round to lowest 500ft increment and shortened (e.g. 17500 -> 175)
        let base = (f64::from(clouds.base) * 3.28084).round() as u32;
        let base = (base - (base % 500)) / 100;
        report += &format!(
            "Cloud conditions {} {}",
            density,
            pronounce_number(base, spoken)
        );
        match clouds.iprecptns {
            1 => report += ", rain",
            2 => report += ", rain and thunderstorm",
            _ => {}
        }
        Some(report)
    } else {
        None
    }
}

static PHONETIC_ALPHABET: &[&str] = &[
    "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliett",
    "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango",
    "Uniform", "Victor", "Whiskey", "X-ray", "Yankee", "Zulu",
];

#[cfg(test)]
mod test {
    use super::*;
    use crate::tts::TextToSpeechProvider;
    use crate::weather::StaticWeather;
    use std::sync::Arc;

    #[test]
    fn test_active_runway() {
        let airfield = Airfield {
            name: String::from("Kutaisi"),
            position: Position {
                x: 0.0,
                y: 0.0,
                alt: 0.0,
            },
            runways: vec![String::from("04"), String::from("22R")],
        };

        assert_eq!(airfield.get_active_runway(0.0), Some("04"));
        assert_eq!(airfield.get_active_runway(30.0), Some("04"));
        assert_eq!(airfield.get_active_runway(129.0), Some("04"));
        assert_eq!(airfield.get_active_runway(311.0), Some("04"));
        assert_eq!(airfield.get_active_runway(180.0), Some("22"));
        assert_eq!(airfield.get_active_runway(270.0), Some("22"));
        assert_eq!(airfield.get_active_runway(309.0), Some("22"));
        assert_eq!(airfield.get_active_runway(131.0), Some("22"));
    }

    #[test]
    fn test_report() {
        let station = Station {
            name: String::from("Kutaisi"),
            atis_freq: 251_000_000,
            traffic_freq: Some(249_500_000),
            tts: TextToSpeechProvider::default(),
            transmitter: Transmitter::Airfield(Airfield {
                name: String::from("Kutaisi"),
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    alt: 0.0,
                },
                runways: vec![String::from("04"), String::from("22")],
            }),
            weather: Arc::new(StaticWeather),
        };

        let report = station.generate_report(26).unwrap().unwrap();
        assert_eq!(report.spoken, "<speak>\nThis is Kutaisi information Alpha. | Runway in use is ZERO 4. | Wind ZERO ZERO 6 at 5 knots. | Temperature 2 2 celcius. | ALTIMETER 2 NINER NINER 7. | Traffic frequency 2 4 NINER DECIMAL 5. | REMARKS. | 1 ZERO 1 5 hectopascal. | QFE 2 NINER NINER 7 or 1 ZERO 1 5. | End information Alpha.\n</speak>");
        assert_eq!(report.textual, "This is Kutaisi information Alpha. Runway in use is 04. Wind 006 at 5 knots. Temperature 22 celcius. ALTIMETER 2997. Traffic frequency 249.5. REMARKS. 1015 hectopascal. QFE 2997 or 1015. End information Alpha.");
    }

    #[test]
    fn test_visibility_report() {
        assert_eq!(get_visibility_report(80_000, true), "Visibility 4 3");
    }

    #[test]
    fn test_clouds_report() {
        fn create_clouds_report(base: u32, density: u32, iprecptns: u32) -> Option<String> {
            let clouds = Clouds {
                base,
                density,
                thickness: 0,
                iprecptns,
            };
            get_clouds_report(&clouds, true)
        }

        assert_eq!(create_clouds_report(8400, 1, 0), None);
        assert_eq!(
            create_clouds_report(8400, 2, 0),
            Some("Cloud conditions few 2 7 5".to_string())
        );
        assert_eq!(
            create_clouds_report(8400, 2, 0),
            Some("Cloud conditions few 2 7 5".to_string())
        );
        assert_eq!(
            create_clouds_report(8500, 6, 1),
            Some("Cloud conditions scattered 2 7 5, rain".to_string())
        );
        assert_eq!(
            create_clouds_report(8500, 10, 2),
            Some("Cloud conditions overcast 2 7 5, rain and thunderstorm".to_string())
        );
    }
}
