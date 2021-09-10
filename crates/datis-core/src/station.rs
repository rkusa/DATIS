use crate::tts::TextToSpeechProvider;
use crate::utils::{pronounce_number, round, round_hundreds};
use crate::weather::WeatherInfo;
pub use srs::message::{LatLngPosition, Position};
use uom::num::Zero;
use uom::si::angle::degree;
use uom::si::f64::{Angle, Length as LengthF64, ThermodynamicTemperature as Temperature};
use uom::si::i32::Length;
use uom::si::length::{foot, meter, nautical_mile};
use uom::si::pressure::{hectopascal, inch_of_mercury};
use uom::si::thermodynamic_temperature::degree_celsius;
use uom::si::velocity::knot;

#[derive(Clone)]
pub struct Station {
    pub name: String,
    pub freq: u64,
    pub tts: TextToSpeechProvider,
    pub transmitter: Transmitter,
    pub ipc: MissionInterface,
}

#[derive(Clone)]
pub enum MissionInterface {
    Static,
    #[cfg(feature = "ipc")]
    Ipc(crate::ipc::MissionRpc),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Transmitter {
    Airfield(Airfield),
    Carrier(Carrier),
    Custom(Custom),
    Weather(WeatherTransmitter),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Airfield {
    pub name: String,
    pub position: Position,
    pub runways: Vec<String>,
    pub traffic_freq: Option<u64>,
    pub info_ltr_offset: usize,
    pub info_ltr_override: Option<char>,
    pub active_rwy_override: Option<String>,
    pub no_hpa: bool,
    pub no_qfe: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Carrier {
    pub name: String,
    pub unit_id: u32,
    pub unit_name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Custom {
    pub position: Option<Position>,
    pub unit_id: u32,
    pub unit_name: String,
    pub message: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WeatherTransmitter {
    pub name: String,
    pub position: Option<Position>,
    pub unit_id: u32,
    pub unit_name: String,
    pub info_ltr_offset: usize,
    pub info_ltr_override: Option<char>,
}

pub struct Report {
    pub textual: String,
    pub spoken: String,
    pub position: LatLngPosition,
}

const SPEAK_START_TAG: &str = "<speak version=\"1.0\" xml:lang=\"en\">\n";

#[inline]
fn break_(spoken: bool) -> &'static str {
    #[cfg(not(test))]
    if spoken {
        "\n"
    } else {
        ""
    }
    #[cfg(test)]
    if spoken {
        "| "
    } else {
        ""
    }
}

fn wind_report(weather: &WeatherInfo, spoken: bool) -> String {
    let wind_dir = format!(
        "{:0>3}",
        weather.wind_dir.get::<degree>().round().to_string()
    );
    format!(
        "{} {} at {} knots. {}",
        if spoken {
            r#"<phoneme alphabet="ipa" ph="w&#618;nd">Wind</phoneme>"#
        } else {
            "Wind"
        },
        pronounce_number(wind_dir, spoken),
        pronounce_number((weather.wind_speed.get::<knot>()).round(), spoken), // to knots
        break_(spoken),
    )
}

fn ceiling_report(weather: &WeatherInfo, alt: Length, spoken: bool) -> String {
    if let Some(ceiling) = weather.get_ceiling(alt) {
        return format!(
            "Ceiling {} {}. {}",
            round_hundreds(ceiling.alt.get::<foot>()),
            ceiling.coverage,
            break_(spoken)
        );
    }

    String::new()
}

fn weather_condition_report(weather: &WeatherInfo, alt: Length, spoken: bool) -> String {
    let conditions = weather.get_weather_conditions(alt);
    if conditions.is_empty() {
        return String::new();
    }

    let ix_last = conditions.len();
    let mut result = String::new();
    for (i, c) in conditions.into_iter().enumerate() {
        result += &format!(
            "{}{}",
            if i == 0 {
                ""
            } else if i == ix_last {
                " and "
            } else {
                ", "
            },
            c
        )
    }

    format!("{}. {}", result, break_(spoken))
}

fn visibility_report(weather: &WeatherInfo, alt: Length, spoken: bool) -> String {
    if let Some(visibility) = weather.get_visibility(alt) {
        if visibility < Length::new::<nautical_mile>(5) {
            let visibility = LengthF64::new::<meter>(visibility.get::<meter>() as f64);
            let visibility = round(visibility.get::<nautical_mile>(), 1);
            return format!(
                "Visibility {}. {}",
                pronounce_number(visibility, spoken),
                break_(spoken)
            );
        }
    }

    String::new()
}

fn temperatur_report(weather: &WeatherInfo, spoken: bool) -> String {
    format!(
        "Temperature {} celcius. {}",
        pronounce_number(
            round(weather.temperature.get::<degree_celsius>(), 1),
            spoken
        ),
        break_(spoken),
    )
}

fn altimeter_report(weather: &WeatherInfo, alt: Length, spoken: bool) -> String {
    format!(
        "ALTIMETER {}. {}",
        // times 100, because we don't want to speak the DECIMAL place
        pronounce_number(
            (weather.get_qnh(alt).get::<inch_of_mercury>() * 100.0).round(),
            spoken
        ),
        break_(spoken),
    )
}

fn hectopascal_report(weather: &WeatherInfo, alt: Length, spoken: bool) -> String {
    format!(
        "{} hectopascal. {}",
        pronounce_number(weather.get_qnh(alt).get::<hectopascal>().round(), spoken),
        break_(spoken),
    )
}

fn qfe_report(weather: &WeatherInfo, spoken: bool) -> String {
    format!(
        "QFE {} {}or {}. {}",
        // times 100, because we don't want to speak the DECIMAL place
        pronounce_number(
            (weather.get_qfe().get::<inch_of_mercury>() * 100.0).round(),
            spoken
        ),
        if spoken {
            // add break to make it easier to mentally process the different numbers
            "<break time=\"500ms\" /> "
        } else {
            ""
        },
        pronounce_number(weather.get_qfe().get::<hectopascal>().round(), spoken),
        break_(spoken),
    )
}

impl Station {
    pub async fn generate_report(&self, report_nr: usize) -> Result<Option<Report>, anyhow::Error> {
        match &self.ipc {
            MissionInterface::Static => self.generate_static_report(report_nr).await,
            #[cfg(feature = "ipc")]
            MissionInterface::Ipc(ipc) => self.generate_report_from_ipc(report_nr, ipc).await,
        }
    }

    #[cfg(feature = "ipc")]
    async fn generate_report_from_ipc(
        &self,
        report_nr: usize,
        ipc: &crate::ipc::MissionRpc,
    ) -> Result<Option<Report>, anyhow::Error> {
        use anyhow::Context;

        match &self.transmitter {
            Transmitter::Airfield(airfield) => {
                let mut weather = ipc
                    .get_weather_at(&airfield.position)
                    .await
                    .context("failed to retrieve weather")?;
                let position = ipc
                    .to_lat_lng(&airfield.position)
                    .await
                    .context("failed to retrieve unit position")?;
                let date = ipc
                    .get_mission_start_date()
                    .await
                    .context("failed to retrieve mission start date")?;
                let declination =
                    igrf::declination(position.lat, position.lng, position.alt as u32, date)
                        .map(|f| f.d)
                        .unwrap_or_else(|err| match err {
                            igrf::Error::DateOutOfRange(f) => f.d,
                            err => {
                                log::error!("Failed to estimate magnetic declination: {}", err);
                                0.0
                            }
                        });

                weather.wind_dir =
                    Angle::new::<degree>((weather.wind_dir.get::<degree>() - declination).floor());

                Ok(Some(Report {
                    textual: airfield.generate_report(
                        report_nr,
                        &weather,
                        Length::new::<meter>(position.alt as i32),
                        false,
                    )?,
                    spoken: airfield.generate_report(
                        report_nr,
                        &weather,
                        Length::new::<meter>(position.alt as i32),
                        true,
                    )?,
                    position,
                }))
            }
            Transmitter::Carrier(unit) => {
                let pos = ipc
                    .get_unit_position(&unit.unit_name)
                    .await
                    .context("failed to retrieve unit position")?;
                let heading = ipc
                    .get_unit_heading(&unit.unit_name)
                    .await
                    .context("failed to retrieve unit heading")?;

                if let (mut pos, Some(heading)) = (pos, heading) {
                    pos.alt = 21.3; // ~70ft carrier deck height
                    let weather = ipc
                        .get_weather_at(&pos)
                        .await
                        .context("failed to retrieve weather")?;
                    let position = ipc
                        .to_lat_lng(&pos)
                        .await
                        .context("failed to retrieve unit position")?;
                    let mission_hour = ipc.get_mission_hour().await?;
                    let date = ipc
                        .get_mission_start_date()
                        .await
                        .context("failed to retrieve mission start date")?;
                    let declination =
                        igrf::declination(position.lat, position.lng, position.alt as u32, date)
                            .map(|f| f.d)
                            .unwrap_or_else(|err| match err {
                                igrf::Error::DateOutOfRange(f) => f.d,
                                err => {
                                    log::error!("Failed to estimate magnetic declination: {}", err);
                                    0.0
                                }
                            });

                    let heading = (heading.to_degrees() - declination).floor() as u16;

                    Ok(Some(Report {
                        textual: unit.generate_report(&weather, heading, mission_hour, false)?,
                        spoken: unit.generate_report(&weather, heading, mission_hour, true)?,
                        position,
                    }))
                } else {
                    Ok(None)
                }
            }
            Transmitter::Custom(custom) => {
                let pos = match &custom.position {
                    Some(pos) => pos.clone(),
                    None => ipc
                        .get_unit_position(&custom.unit_name)
                        .await
                        .context("failed to retrieve unit position")?,
                };

                let position = ipc
                    .to_lat_lng(&pos)
                    .await
                    .context("failed to retrieve unit position")?;

                Ok(Some(Report {
                    textual: custom.message.clone(),
                    spoken: format!(
                        "<speak version=\"1.0\" xml:lang=\"en\">{}</speak>",
                        escape_xml(&custom.message)
                    ),
                    position,
                }))
            }
            Transmitter::Weather(weather) => {
                let pos = match &weather.position {
                    Some(pos) => pos.clone(),
                    None => ipc
                        .get_unit_position(&weather.unit_name)
                        .await
                        .context("failed to retrieve unit position")?,
                };

                let weather_info = ipc
                    .get_weather_at(&pos)
                    .await
                    .context("failed to retrieve weather")?;
                let position = ipc
                    .to_lat_lng(&pos)
                    .await
                    .context("failed to convert unit position to lat lng")?;

                Ok(Some(Report {
                    textual: weather.generate_report(
                        report_nr,
                        &weather_info,
                        Length::new::<meter>(position.alt as i32),
                        false,
                    )?,
                    spoken: weather.generate_report(
                        report_nr,
                        &weather_info,
                        Length::new::<meter>(position.alt as i32),
                        true,
                    )?,
                    position,
                }))
            }
        }
    }

    async fn generate_static_report(
        &self,
        report_nr: usize,
    ) -> Result<Option<Report>, anyhow::Error> {
        use uom::si::f64::{Pressure, Velocity};
        use uom::si::pressure::pascal;

        let weather_info = WeatherInfo {
            clouds: None,
            wind_speed: Velocity::new::<knot>(5.0),
            wind_dir: Angle::new::<degree>(330.0),
            temperature: Temperature::new::<degree_celsius>(22.0),
            pressure_sealevel: Pressure::new::<pascal>(101_500.0),
            pressure_groundlevel: Pressure::new::<pascal>(101_500.0),
            position: Position::default(),
            ..Default::default()
        };

        match &self.transmitter {
            Transmitter::Airfield(airfield) => Ok(Some(Report {
                textual: airfield.generate_report(
                    report_nr,
                    &weather_info,
                    Length::zero(),
                    false,
                )?,
                spoken: airfield.generate_report(report_nr, &weather_info, Length::zero(), true)?,
                position: LatLngPosition::default(),
            })),
            Transmitter::Carrier(unit) => {
                let heading = 180;
                let mission_hour = 7;

                Ok(Some(Report {
                    textual: unit.generate_report(&weather_info, heading, mission_hour, false)?,
                    spoken: unit.generate_report(&weather_info, heading, mission_hour, true)?,
                    position: LatLngPosition::default(),
                }))
            }
            Transmitter::Custom(custom) => Ok(Some(Report {
                textual: custom.message.clone(),
                spoken: format!(
                    "<speak version=\"1.0\" xml:lang=\"en\">{}</speak>",
                    escape_xml(&custom.message)
                ),
                position: LatLngPosition::default(),
            })),
            Transmitter::Weather(weather) => Ok(Some(Report {
                textual: weather.generate_report(
                    report_nr,
                    &weather_info,
                    Length::zero(),
                    false,
                )?,
                spoken: weather.generate_report(report_nr, &weather_info, Length::zero(), true)?,
                position: LatLngPosition::default(),
            })),
        }
    }
}

impl Airfield {
    fn get_active_runway(&self, wind_dir: Angle) -> Option<&str> {
        if let Some(rwy_override) = &self.active_rwy_override {
            return Some(rwy_override);
        }

        let lr: &[_] = &['L', 'R'];
        for rwy in &self.runways {
            let rwy = rwy.trim_matches(lr);
            if let Ok(mut rwy_dir) = rwy.parse::<f64>() {
                rwy_dir *= 10.0; // e.g. 04 to 040
                let phi = (wind_dir.get::<degree>() - rwy_dir).abs() % 360.0;
                let distance = if phi > 180.0 { 360.0 - phi } else { phi };
                if distance <= 90.0 {
                    return Some(rwy);
                }
            } else {
                log::error!("Error parsing runway: {}", rwy);
            }
        }

        None
    }

    pub fn generate_report(
        &self,
        report_nr: usize,
        weather: &WeatherInfo,
        alt: Length,
        spoken: bool,
    ) -> Result<String, anyhow::Error> {
        let mut report = if spoken { SPEAK_START_TAG } else { "" }.to_string();

        let information_num = if let Some(ltr_override) = self.info_ltr_override {
            (ltr_override.to_ascii_uppercase() as usize) - 65
        } else {
            self.info_ltr_offset + report_nr
        };
        let information_letter = phonetic_alphabet::lookup(information_num);

        report += &format!(
            "This is {} information {}. {}",
            self.name,
            information_letter,
            break_(spoken)
        );

        if let Some(rwy) = self.get_active_runway(weather.wind_dir) {
            let rwy = pronounce_number(rwy, spoken);
            report += &format!("Runway in use is {}. {}", rwy, break_(spoken));
        } else {
            log::error!("Could not find active runway for {}", self.name);
        }

        if let Some(traffic_freq) = self.traffic_freq {
            report += &format!(
                "Traffic frequency {}. {}",
                pronounce_number(round(traffic_freq as f64 / 1_000_000.0, 3), spoken),
                break_(spoken)
            );
        }

        report += &wind_report(weather, spoken);
        report += &ceiling_report(weather, alt, spoken);
        report += &weather_condition_report(weather, alt, spoken);
        report += &visibility_report(weather, alt, spoken);
        report += &temperatur_report(weather, spoken);
        report += &altimeter_report(weather, alt, spoken);

        if !self.no_hpa || !self.no_qfe {
            report += &format!("REMARKS. {}", break_(spoken));
        }

        if !self.no_hpa {
            report += &hectopascal_report(weather, alt, spoken);
        }

        if !self.no_qfe {
            report += &qfe_report(weather, spoken);
        }

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
        heading: u16,
        mission_hour: u16,
        spoken: bool,
    ) -> Result<String, anyhow::Error> {
        let mut report = if spoken { SPEAK_START_TAG } else { "" }.to_string();

        report += &format!("99, {}", break_(spoken));

        let wind_dir = format!(
            "{:0>3}",
            weather.wind_dir.get::<degree>().round().to_string()
        );
        report += &format!(
            r#"{}'s {} {} at {} knots, {}"#,
            self.name,
            if spoken {
                r#"<phoneme alphabet="ipa" ph="w&#618;nd">wind</phoneme>"#
            } else {
                "wind"
            },
            pronounce_number(wind_dir, spoken),
            pronounce_number((weather.wind_speed.get::<knot>()).round(), spoken),
            break_(spoken),
        );

        let alt = Length::new::<foot>(70); // carrier deck alt
        report += &altimeter_report(weather, alt, spoken);

        // Case 1: daytime, ceiling >= 3000ft; visibility distance >= 5nm
        // Case 2: daytime, ceiling >= 1000ft; visibility distance >= 5nm
        // Case 3: nighttime or daytime, ceiling < 1000ft and visibility distance <= 5nm
        let mut case = 1;
        if let Some(ceiling) = weather.get_ceiling(alt) {
            if ceiling.alt < Length::new::<foot>(1_000) {
                case = 3;
            } else if ceiling.alt < Length::new::<foot>(3_000) {
                case = 2;
            }
        }

        if let Some(visibility) = weather.get_visibility(alt) {
            if visibility < Length::new::<nautical_mile>(5) {
                case = 3;
            }
        }

        // night time is only estimated, it could be improved by somehow taking the different time-
        // zones of the different maps and the mission date into account.
        if mission_hour >= 21 || mission_hour <= 5 {
            case = 3;
        }

        report += &format!("CASE {}, {}", case, break_(spoken),);

        let brc = heading;
        let mut fh = heading - 9; // 9 -> 9deg angled deck
        if fh > 360 {
            fh -= 360;
        }

        let brc = format!("{:0>3}", brc);
        report += &format!("BRC {}, {}", pronounce_number(brc, spoken), break_(spoken));

        let fh = format!("{:0>3}", fh);
        report += &format!(
            "expected final heading {}, {}",
            pronounce_number(fh, spoken),
            break_(spoken),
        );

        report += "report initial.";

        if spoken {
            report += "\n</speak>";
        }

        Ok(report)
    }
}

impl WeatherTransmitter {
    pub fn generate_report(
        &self,
        report_nr: usize,
        weather: &WeatherInfo,
        alt: Length,
        spoken: bool,
    ) -> Result<String, anyhow::Error> {
        let information_num = if let Some(ltr_override) = self.info_ltr_override {
            (ltr_override.to_ascii_uppercase() as usize) - 65
        } else {
            self.info_ltr_offset + report_nr
        };
        let information_letter = phonetic_alphabet::lookup(information_num);
        let mut report = if spoken { SPEAK_START_TAG } else { "" }.to_string();

        report += &format!(
            "This is weather station {} information {}. {}",
            self.name,
            information_letter,
            break_(spoken)
        );

        report += &wind_report(weather, spoken);
        report += &ceiling_report(weather, alt, spoken);
        report += &weather_condition_report(weather, alt, spoken);
        report += &visibility_report(weather, alt, spoken);
        report += &temperatur_report(weather, spoken);
        report += &altimeter_report(weather, alt, spoken);

        report += &format!("REMARKS. {}", break_(spoken),);
        report += &hectopascal_report(weather, alt, spoken);
        report += &qfe_report(weather, spoken);

        report += &format!("End information {}.", information_letter);

        if spoken {
            report += "\n</speak>";
        }

        Ok(report)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('<', "&lt;").replace('&', "&amp;")
}

mod phonetic_alphabet {
    static PHONETIC_ALPHABET: &[&str] = &[
        "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India",
        "Juliett", "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo",
        "Sierra", "Tango", "Uniform", "Victor", "Whiskey", "X-ray", "Yankee", "Zulu",
    ];

    pub fn lookup(idx: usize) -> &'static str {
        PHONETIC_ALPHABET[idx % PHONETIC_ALPHABET.len()]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tts::TextToSpeechProvider;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_active_runway() {
        let airfield = Airfield {
            name: String::from("Kutaisi"),
            position: Position::default(),
            runways: vec![String::from("04"), String::from("22R")],
            traffic_freq: None,
            info_ltr_offset: 0,
            info_ltr_override: None,
            active_rwy_override: None,
            no_hpa: false,
            no_qfe: false,
        };

        assert_eq!(
            airfield.get_active_runway(Angle::new::<degree>(0.0)),
            Some("04")
        );
        assert_eq!(
            airfield.get_active_runway(Angle::new::<degree>(30.0)),
            Some("04")
        );
        assert_eq!(
            airfield.get_active_runway(Angle::new::<degree>(129.0)),
            Some("04")
        );
        assert_eq!(
            airfield.get_active_runway(Angle::new::<degree>(311.0)),
            Some("04")
        );
        assert_eq!(
            airfield.get_active_runway(Angle::new::<degree>(180.0)),
            Some("22")
        );
        assert_eq!(
            airfield.get_active_runway(Angle::new::<degree>(270.0)),
            Some("22")
        );
        assert_eq!(
            airfield.get_active_runway(Angle::new::<degree>(309.0)),
            Some("22")
        );
        assert_eq!(
            airfield.get_active_runway(Angle::new::<degree>(131.0)),
            Some("22")
        );
    }

    #[tokio::test]
    async fn test_atis_report() {
        let station = Station {
            name: String::from("Kutaisi"),
            freq: 251_000_000,
            tts: TextToSpeechProvider::default(),
            transmitter: Transmitter::Airfield(Airfield {
                name: String::from("Kutaisi"),
                position: Position::default(),
                runways: vec![String::from("04"), String::from("22")],
                traffic_freq: Some(249_500_000),
                info_ltr_offset: 0,
                info_ltr_override: None,
                active_rwy_override: None,
                no_hpa: false,
                no_qfe: false,
            }),
            ipc: MissionInterface::Static,
        };

        let report = station.generate_report(26).await.unwrap().unwrap();
        assert_eq!(report.spoken, "<speak version=\"1.0\" xml:lang=\"en\">\nThis is Kutaisi information Alpha. | Runway in use is ZERO 4. | Traffic frequency 2 4 NINER DECIMAL 5. | <phoneme alphabet=\"ipa\" ph=\"w&#618;nd\">Wind</phoneme> 3 3 ZERO at 5 knots. | Temperature 2 2 celcius. | ALTIMETER 2 NINER NINER 7. | REMARKS. | 1 ZERO 1 5 hectopascal. | QFE 2 NINER NINER 7 <break time=\"500ms\" /> or 1 ZERO 1 5. | End information Alpha.\n</speak>");
        assert_eq!(report.textual, "This is Kutaisi information Alpha. Runway in use is 04. Traffic frequency 249.5. Wind 330 at 5 knots. Temperature 22 celcius. ALTIMETER 2997. REMARKS. 1015 hectopascal. QFE 2997 or 1015. End information Alpha.");
    }

    #[tokio::test]
    async fn test_report_with_info_letter_offset() {
        let station = Station {
            name: String::from("Kutaisi"),
            freq: 251_000_000,
            tts: TextToSpeechProvider::default(),
            transmitter: Transmitter::Airfield(Airfield {
                name: String::from("Kutaisi"),
                position: Position::default(),
                runways: vec![String::from("04"), String::from("22")],
                traffic_freq: Some(249_500_000),
                info_ltr_offset: 15, // Should be "Papa"
                info_ltr_override: None,
                active_rwy_override: None,
                no_hpa: false,
                no_qfe: false,
            }),
            ipc: MissionInterface::Static,
        };

        let report = station.generate_report(26).await.unwrap().unwrap();
        assert_eq!(report.spoken, "<speak version=\"1.0\" xml:lang=\"en\">\nThis is Kutaisi information Papa. | Runway in use is ZERO 4. | Traffic frequency 2 4 NINER DECIMAL 5. | <phoneme alphabet=\"ipa\" ph=\"w&#618;nd\">Wind</phoneme> 3 3 ZERO at 5 knots. | Temperature 2 2 celcius. | ALTIMETER 2 NINER NINER 7. | REMARKS. | 1 ZERO 1 5 hectopascal. | QFE 2 NINER NINER 7 <break time=\"500ms\" /> or 1 ZERO 1 5. | End information Papa.\n</speak>");
        assert_eq!(report.textual, "This is Kutaisi information Papa. Runway in use is 04. Traffic frequency 249.5. Wind 330 at 5 knots. Temperature 22 celcius. ALTIMETER 2997. REMARKS. 1015 hectopascal. QFE 2997 or 1015. End information Papa.");
    }

    #[tokio::test]
    async fn test_report_with_info_letter_override() {
        let station = Station {
            name: String::from("Kutaisi"),
            freq: 251_000_000,
            tts: TextToSpeechProvider::default(),
            transmitter: Transmitter::Airfield(Airfield {
                name: String::from("Kutaisi"),
                position: Position::default(),
                runways: vec![String::from("04"), String::from("22")],
                traffic_freq: Some(249_500_000),
                info_ltr_offset: 15,
                info_ltr_override: Some('Q'),
                active_rwy_override: None,
                no_hpa: false,
                no_qfe: false,
            }),
            ipc: MissionInterface::Static,
        };

        let report = station.generate_report(26).await.unwrap().unwrap();
        assert_eq!(report.spoken, "<speak version=\"1.0\" xml:lang=\"en\">\nThis is Kutaisi information Quebec. | Runway in use is ZERO 4. | Traffic frequency 2 4 NINER DECIMAL 5. | <phoneme alphabet=\"ipa\" ph=\"w&#618;nd\">Wind</phoneme> 3 3 ZERO at 5 knots. | Temperature 2 2 celcius. | ALTIMETER 2 NINER NINER 7. | REMARKS. | 1 ZERO 1 5 hectopascal. | QFE 2 NINER NINER 7 <break time=\"500ms\" /> or 1 ZERO 1 5. | End information Quebec.\n</speak>");
        assert_eq!(report.textual, "This is Kutaisi information Quebec. Runway in use is 04. Traffic frequency 249.5. Wind 330 at 5 knots. Temperature 22 celcius. ALTIMETER 2997. REMARKS. 1015 hectopascal. QFE 2997 or 1015. End information Quebec.");
    }

    #[test]
    fn test_phonetic_alpha_lookup() {
        assert_eq!(phonetic_alphabet::lookup(0), "Alpha");
        assert_eq!(phonetic_alphabet::lookup(14), "Oscar");
        assert_eq!(phonetic_alphabet::lookup(25), "Zulu");
        //It should also wrap around if the idx is higher than 25.
        assert_eq!(phonetic_alphabet::lookup(26), "Alpha");
        assert_eq!(phonetic_alphabet::lookup(40), "Oscar");
        assert_eq!(phonetic_alphabet::lookup(51), "Zulu");
    }

    #[tokio::test]
    async fn test_carrier_report() {
        let station = Station {
            name: String::from("Mother"),
            freq: 251_000_000,
            tts: TextToSpeechProvider::default(),
            transmitter: Transmitter::Carrier(Carrier {
                name: "Stennis".to_string(),
                unit_id: 42,
                unit_name: "Stennis".to_string(),
            }),
            ipc: MissionInterface::Static,
        };

        let report = station.generate_report(26).await.unwrap().unwrap();
        assert_eq!(report.spoken, "<speak version=\"1.0\" xml:lang=\"en\">\n99, | Stennis's <phoneme alphabet=\"ipa\" ph=\"w&#618;nd\">wind</phoneme> 3 3 ZERO at 5 knots, | ALTIMETER 3 ZERO ZERO 5. | CASE 1, | BRC 1 8 ZERO, | expected final heading 1 7 1, | report initial.\n</speak>");
        assert_eq!(report.textual, "99, Stennis's wind 330 at 5 knots, ALTIMETER 3005. CASE 1, BRC 180, expected final heading 171, report initial.");
    }

    #[tokio::test]
    async fn test_custom_broadcast_report() {
        let station = Station {
            name: String::from("Broadcast station"),
            freq: 251_000_000,
            tts: TextToSpeechProvider::default(),
            transmitter: Transmitter::Custom(Custom {
                position: Some(Position::default()),
                unit_id: 42,
                unit_name: "Soldier".to_string(),
                message: "Hello world".to_string(),
            }),
            ipc: MissionInterface::Static,
        };

        let report = station.generate_report(26).await.unwrap().unwrap();
        assert_eq!(
            report.spoken,
            "<speak version=\"1.0\" xml:lang=\"en\">Hello world</speak>"
        );
        assert_eq!(report.textual, "Hello world");
    }

    #[tokio::test]
    async fn test_weather_report() {
        let station = Station {
            name: String::from("Mother"),
            freq: 251_000_000,
            tts: TextToSpeechProvider::default(),
            transmitter: Transmitter::Weather(WeatherTransmitter {
                name: "Mountain Range".to_string(),
                position: Some(Position::default()),
                unit_id: 42,
                unit_name: "Weather Post".to_string(),
                info_ltr_offset: 15, // Should be "Papa",
                info_ltr_override: None,
            }),
            ipc: MissionInterface::Static,
        };

        let report = station.generate_report(26).await.unwrap().unwrap();
        assert_eq!(report.spoken, "<speak version=\"1.0\" xml:lang=\"en\">\nThis is weather station Mountain Range information Papa. | <phoneme alphabet=\"ipa\" ph=\"w&#618;nd\">Wind</phoneme> 3 3 ZERO at 5 knots. | Temperature 2 2 celcius. | ALTIMETER 2 NINER NINER 7. | REMARKS. | 1 ZERO 1 5 hectopascal. | QFE 2 NINER NINER 7 <break time=\"500ms\" /> or 1 ZERO 1 5. | End information Papa.\n</speak>");
        assert_eq!(report.textual, "This is weather station Mountain Range information Papa. Wind 330 at 5 knots. Temperature 22 celcius. ALTIMETER 2997. REMARKS. 1015 hectopascal. QFE 2997 or 1015. End information Papa.");
    }
}
