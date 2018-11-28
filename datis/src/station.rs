use crate::error::Error;
use crate::tts::VoiceKind;
use crate::utils::{pronounce_number, round};
use crate::weather::{DynamicWeather, StaticWeather, WeatherInfo, WeatherKind};

#[derive(Debug, Clone)]
pub struct Station {
    pub name: String,
    pub atis_freq: u64,
    pub traffic_freq: Option<u64>,
    pub voice: VoiceKind,
    pub airfield: Airfield,
    pub weather_kind: WeatherKind,
    pub static_weather: StaticWeather,
    pub dynamic_weather: DynamicWeather,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    #[serde(rename = "z")]
    pub y: f64,
    #[serde(rename = "y")]
    pub alt: f64,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Airfield {
    pub name: String,
    pub position: Position,
    pub runways: Vec<String>,
}

#[cfg(not(test))]
pub(crate) const BREAK: &str = "<break time=\"500ms\"/>\n";
#[cfg(test)]
pub(crate) const BREAK: &str = "| ";

impl Station {
    pub fn generate_report(&self, report_nr: usize) -> Result<String, Error> {
        let information_letter = PHONETIC_ALPHABET[report_nr % PHONETIC_ALPHABET.len()];

        let weather = self.get_current_weather()?;
        let mut report = format!(
            "<speak>\nThis is {} information {}. {}",
            self.name, information_letter, BREAK
        );

        if let Some(rwy) = self.get_active_runway(weather.wind_dir) {
            let rwy = pronounce_number(rwy);
            report += &format!("Runway in use is {}. {}", rwy, BREAK);
        } else {
            error!("Could not find active runway for {}", self.name);
        }

        let wind_dir = format!("{:0>3}", weather.wind_dir.round().to_string());
        report += &format!(
            "Wind {} at {} knots. {}",
            pronounce_number(wind_dir),
            pronounce_number((weather.wind_speed * 1.94384).round()), // to knots
            BREAK,
        );

        if self.weather_kind == WeatherKind::Static {
            report += &self.static_weather.get_clouds_report();
        }

        report += &format!(
            "Temperature {} celcius. {}",
            pronounce_number(round(weather.temperature, 1)),
            BREAK,
        );

        report += &format!(
            "ALTIMETER {}. {}",
            // inHg, but using 0.02953 instead of 0.0002953 since we don't want to speak the
            // DECIMAL here
            pronounce_number((weather.pressure_qnh * 0.02953).round()),
            BREAK,
        );

        if let Some(traffic_freq) = self.traffic_freq {
            report += &format!(
                "Traffic frequency {}. {}",
                pronounce_number(round(traffic_freq as f64 / 1_000_000.0, 3)),
                BREAK
            );
        }

        report += &format!("REMARKS. {}", BREAK,);
        report += &format!(
            "{} hectopascal. {}",
            pronounce_number((weather.pressure_qnh / 100.0).round()), // to hPA
            BREAK,
        );
        report += &format!(
            "QFE {} or {}. {}",
            pronounce_number((weather.pressure_qfe * 0.02953).round()), // to inHg
            pronounce_number((weather.pressure_qfe / 100.0).round()),   // to hPA
            BREAK,
        );

        report += &format!("End information {}.\n</speak>", information_letter);

        Ok(report)
    }

    #[cfg(test)]
    fn get_current_weather(&self) -> Result<WeatherInfo, Error> {
        Ok(WeatherInfo {
            wind_speed: 5.0,
            wind_dir: (330.0f64).to_radians(),
            temperature: 22.0,
            pressure_qnh: 101500.0,
            pressure_qfe: 101500.0,
        })
    }

    #[cfg(not(test))]
    fn get_current_weather(&self) -> Result<WeatherInfo, Error> {
        let info = self.dynamic_weather.get_at(
            self.airfield.position.x,
            self.airfield.position.y,
            self.airfield.position.alt,
        )?;

        Ok(info)
    }

    fn get_active_runway(&self, wind_dir: f64) -> Option<&str> {
        for rwy in &self.airfield.runways {
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
}

static PHONETIC_ALPHABET: &'static [&str] = &[
    "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliett",
    "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango",
    "Uniform", "Victor", "Whiskey", "X-ray", "Yankee", "Zulu",
];

#[cfg(test)]
mod test {
    use super::{Airfield, Position, Station};
    use crate::tts::VoiceKind;
    use crate::weather::{DynamicWeather, StaticWeather, WeatherKind};

    #[test]
    fn test_active_runway() {
        let station = Station {
            name: String::from("Kutaisi"),
            atis_freq: 251_000_000,
            traffic_freq: None,
            voice: VoiceKind::StandardC,
            airfield: Airfield {
                name: String::from("Kutaisi"),
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    alt: 0.0,
                },
                runways: vec![String::from("04"), String::from("22")],
            },
            weather_kind: WeatherKind::Static,
            static_weather: StaticWeather::default(),
            dynamic_weather: DynamicWeather::create("").unwrap(),
        };

        assert_eq!(station.get_active_runway(0.0), Some("04"));
        assert_eq!(station.get_active_runway(30.0), Some("04"));
        assert_eq!(station.get_active_runway(129.0), Some("04"));
        assert_eq!(station.get_active_runway(311.0), Some("04"));
        assert_eq!(station.get_active_runway(180.0), Some("22"));
        assert_eq!(station.get_active_runway(270.0), Some("22"));
        assert_eq!(station.get_active_runway(309.0), Some("22"));
        assert_eq!(station.get_active_runway(131.0), Some("22"));
    }

    #[test]
    fn test_report() {
        let station = Station {
            name: String::from("Kutaisi"),
            atis_freq: 251_000_000,
            traffic_freq: Some(249_500_000),
            voice: VoiceKind::StandardC,
            airfield: Airfield {
                name: String::from("Kutaisi"),
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    alt: 0.0,
                },
                runways: vec![String::from("04"), String::from("22")],
            },
            weather_kind: WeatherKind::Static,
            static_weather: StaticWeather::default(),
            dynamic_weather: DynamicWeather::create("").unwrap(),
        };

        let report = station.generate_report(26).unwrap();
        assert_eq!(report, "<speak>\nThis is Kutaisi information Alpha. | Runway in use is ZERO 4. | Wind ZERO ZERO 6 at 1 ZERO knots. | Visibility ZERO. | Temperature 2 2 celcius. | ALTIMETER 2 NINER NINER 7. | Traffic frequency 2 4 NINER DECIMAL 5. | REMARKS. | 1 ZERO 1 5 hectopascal. | QFE 2 NINER NINER 7 or 1 ZERO 1 5. | End information Alpha.\n</speak>");
    }
}
