use crate::rpc::{Clouds, MissionRpc, WeatherInfo};
use crate::tts::TextToSpeechProvider;
use crate::utils::{m_to_ft, m_to_nm, pronounce_number, round};
pub use srs::message::Position;

#[cfg(not(feature = "static-weather"))]
use anyhow::Context;

#[derive(Clone)]
pub struct Station {
    pub name: String,
    pub freq: u64,
    pub tts: TextToSpeechProvider,
    pub transmitter: Transmitter,
    pub rpc: Option<MissionRpc>,
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

const SPEAK_START_TAG: &str = "<speak version=\"1.0\" xml:lang=\"en-US\">\n";

impl Station {
    #[cfg(not(feature = "static-weather"))]
    pub async fn generate_report(&self, report_nr: usize) -> Result<Option<Report>, anyhow::Error> {
        match (self.rpc.as_ref(), &self.transmitter) {
            (Some(rpc), Transmitter::Airfield(airfield)) => {
                let weather = rpc
                    .get_weather_at(
                        airfield.position.x,
                        airfield.position.y,
                        airfield.position.alt,
                    )
                    .await
                    .context("failed to retrieve weather")?;

                Ok(Some(Report {
                    textual: airfield.generate_report(report_nr, &weather, false)?,
                    spoken: airfield.generate_report(report_nr, &weather, true)?,
                    position: airfield.position.clone(),
                }))
            }
            (Some(rpc), Transmitter::Carrier(unit)) => {
                let pos = rpc
                    .get_unit_position(&unit.unit_name)
                    .await
                    .context("failed to retrieve unit position")?;
                let heading = rpc
                    .get_unit_heading(&unit.unit_name)
                    .await
                    .context("failed to retrieve unit heading")?;

                if let (Some(pos), Some(heading)) = (pos, heading) {
                    let weather = rpc
                        .get_weather_at(pos.x, pos.y, pos.alt)
                        .await
                        .context("failed to retrieve weather")?;
                    let mission_hour = rpc.get_mission_hour().await?;

                    Ok(Some(Report {
                        textual: unit.generate_report(&weather, heading, mission_hour, false)?,
                        spoken: unit.generate_report(&weather, heading, mission_hour, true)?,
                        position: pos,
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    #[cfg(feature = "static-weather")]
    pub async fn generate_report(&self, report_nr: usize) -> Result<Option<Report>, anyhow::Error> {
        let weather = WeatherInfo {
            clouds: None,
            visibility: None,
            wind_speed: 5.0,
            wind_dir: (330.0f64).to_radians(),
            temperature: 22.0,
            pressure_qnh: 101_500.0,
            pressure_qfe: 101_500.0,
        };

        match &self.transmitter {
            Transmitter::Airfield(airfield) => Ok(Some(Report {
                textual: airfield.generate_report(report_nr, &weather, false)?,
                spoken: airfield.generate_report(report_nr, &weather, true)?,
                position: airfield.position.clone(),
            })),
            Transmitter::Carrier(unit) => {
                let pos = Position::default();
                let heading = 180.0;
                let mission_hour = 7;

                Ok(Some(Report {
                    textual: unit.generate_report(&weather, heading, mission_hour, false)?,
                    spoken: unit.generate_report(&weather, heading, mission_hour, true)?,
                    position: pos,
                }))
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
        let mut report = if spoken { SPEAK_START_TAG } else { "" }.to_string();

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

        let mut visibility = None;
        if let Some(ref clouds_report) = weather.clouds {
            if self.position.alt > clouds_report.base as f64
                && self.position.alt < clouds_report.base as f64 + clouds_report.thickness as f64
                && clouds_report.density >= 9
            {
                // the airport is within completely condensed clouds
                visibility = Some(0);
            }
        }

        if let Some(visibility) = visibility.or(weather.visibility) {
            // 9260 m = 5 nm
            if visibility < 9_260 {
                report += &format!("{}. {}", get_visibility_report(visibility, spoken), _break);
            }
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
        heading: f64,
        mission_hour: u16,
        spoken: bool,
    ) -> Result<String, anyhow::Error> {
        #[cfg(not(test))]
        let _break = if spoken { "\n" } else { "" };
        #[cfg(test)]
        let _break = if spoken { "| " } else { "" };

        let mut report = if spoken { SPEAK_START_TAG } else { "" }.to_string();

        report += &format!("{}, {}", pronounce_number(99, spoken), _break);

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

        // Case 1: daytime, ceiling >= 3000ft; visibility distance >= 5nm
        // Case 2: daytime, ceiling >= 1000ft; visibility distance >= 5nm
        // Case 3: nighttime or daytime, ceiling < 1000ft and visibility distance <= 5nm
        let mut case = 1;
        if let Some(ceiling) = weather.clouds.as_ref().map(|clouds| clouds.base) {
            let ft = m_to_ft(ceiling as f64);
            if ft < 1_000.0 {
                case = 3;
            } else if ft < 3_000.0 {
                case = 2;
            }
        }

        if let Some(visibility) = weather.visibility {
            // 9260 m = 5 nm
            if visibility < 9_260 {
                case = 3;
            }
        }

        // night time is only estimated, it could be improved by somehow taking the different time-
        // zones of the different maps and the mission date into account.
        if mission_hour >= 21 || mission_hour <= 5 {
            case = 3;
        }

        report += &format!("CASE {}, {}", case, _break,);

        let brc = heading.to_degrees().round();
        let mut fh = brc - 9.0; // 9 -> 9deg angled deck
        if fh < 0.0 {
            fh += 360.0;
        }

        let brc = format!("{:0>3}", brc);
        report += &format!("BRC {}, {}", pronounce_number(brc, spoken), _break,);

        let fh = format!("{:0>3}", fh);
        report += &format!(
            "expected final heading {}, {}",
            pronounce_number(fh, spoken),
            _break,
        );

        report += "report initial.";

        if spoken {
            report += "\n</speak>";
        }

        Ok(report)
    }
}

fn get_visibility_report(visibility: u32, spoken: bool) -> String {
    let visibility = round(m_to_nm(f64::from(visibility)), 1);
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
        let base = m_to_ft(f64::from(clouds.base)).round() as u32;
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
            traffic_freq: None,
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

    #[tokio::test]
    async fn test_report() {
        let station = Station {
            name: String::from("Kutaisi"),
            freq: 251_000_000,
            tts: TextToSpeechProvider::default(),
            transmitter: Transmitter::Airfield(Airfield {
                name: String::from("Kutaisi"),
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    alt: 0.0,
                },
                runways: vec![String::from("04"), String::from("22")],
                traffic_freq: Some(249_500_000),
            }),
            rpc: None,
        };

        let report = station.generate_report(26).await.unwrap().unwrap();
        assert_eq!(report.spoken, "<speak version=\"1.0\" xml:lang=\"en-US\">\nThis is Kutaisi information Alpha. | Runway in use is ZERO 4. | Wind ZERO ZERO 6 at 5 knots. | Temperature 2 2 celcius. | ALTIMETER 2 NINER NINER 7. | Traffic frequency 2 4 NINER DECIMAL 5. | REMARKS. | 1 ZERO 1 5 hectopascal. | QFE 2 NINER NINER 7 or 1 ZERO 1 5. | End information Alpha.\n</speak>");
        assert_eq!(report.textual, "This is Kutaisi information Alpha. Runway in use is 04. Wind 006 at 5 knots. Temperature 22 celcius. ALTIMETER 2997. Traffic frequency 249.5. REMARKS. 1015 hectopascal. QFE 2997 or 1015. End information Alpha.");
    }

    #[test]
    fn test_visibility_report() {
        assert_eq!(get_visibility_report(6_000, true), "Visibility 3 DECIMAL 2");
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
