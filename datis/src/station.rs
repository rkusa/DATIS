use crate::error::Error;
use crate::weather::{DynamicWeather, WeatherInfo};

#[derive(Debug, Clone)]
pub struct Station {
    pub name: String,
    pub atis_freq: u64,
    pub traffic_freq: Option<u64>,
    pub airfield: Airfield,
    pub static_wind: Option<StaticWind>,
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

#[derive(Debug, PartialEq, Clone)]
pub struct StaticWind {
    pub dir: f64,
    pub speed: f64,
}

impl Station {
    pub fn generate_report(&self, report_nr: usize) -> Result<String, Error> {
        let information_letter = PHONETIC_ALPHABET[report_nr % PHONETIC_ALPHABET.len()];

        let weather = self.get_current_weather()?;
        let mut report = format!("This is {} information {}. ", self.name, information_letter);

        if let Some(rwy) = self.get_active_runway(weather.wind_dir) {
            let rwy = pronounce_number(rwy);
            report += &format!("Runway in use is {}. ", rwy);
        } else {
            error!("Could not find active runway for {}", self.name);
        }

        let wind_dir = format!("{:0>3}", weather.wind_dir.to_degrees().round().to_string());
        report += &format!(
            "Wind {} at {} knots. ",
            pronounce_number(wind_dir),
            pronounce_number((weather.wind_speed * 1.94384).round()), // to knots
        );

        report += &format!(
            "Temperature {} celcius, ALTIMETER {}. ",
            pronounce_number(round(weather.temperature, 1)),
            pronounce_number(round(weather.pressure * 0.0002953, 2)), // inHg
        );

        if let Some(traffic_freq) = self.traffic_freq {
            report += &format!(
                "Traffic frequency {}. ",
                pronounce_number(round(traffic_freq as f64 / 1_000_000.0, 3))
            );
        }

        report += &format!(
            "REMARK {} hectopascal. ",
            pronounce_number((weather.pressure / 100.0).round()), // to hPA
        );

        report += &format!("End information {}. ", information_letter);

        Ok(report)
    }

    #[cfg(test)]
    fn get_current_weather(&self) -> Result<WeatherInfo, Error> {
        Ok(WeatherInfo {
            wind_speed: 5.0,
            wind_dir: (330.0f64).to_radians(),
            temperature: 22.0,
            pressure: 1015.0,
        })
    }

    #[cfg(not(test))]
    fn get_current_weather(&self) -> Result<WeatherInfo, Error> {
        let mut info = self.dynamic_weather.get_at(
            self.airfield.position.x,
            self.airfield.position.y,
            self.airfield.position.alt,
        )?;

        if let Some(ref static_wind) = self.static_wind {
            info.wind_speed = static_wind.speed;
            info.wind_dir = static_wind.dir;
        }

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

fn round(n: f64, max_decimal_places: i32) -> f64 {
    if max_decimal_places == 0 {
        return n.round();
    }
    let m = (10.0f64).powi(max_decimal_places);
    (n * m).round() / m
}

fn pronounce_number<S>(n: S) -> String
where
    S: ToString,
{
    n.to_string()
        .chars()
        .filter_map(|c| match c {
            '.' => Some(String::from("decimal")),
            _ => Some(c.to_string()),
        })
        .collect::<Vec<String>>()
        .join(" ")
}

static PHONETIC_ALPHABET: &'static [&str] = &[
    "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliett",
    "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango",
    "Uniform", "Victor", "Whiskey", "X-ray", "Yankee", "Zulu",
];

#[cfg(test)]
mod test {
    use super::{Airfield, Position, Station};
    use crate::weather::DynamicWeather;
    use hlua51::Lua;
    use std::cell::RefCell;

    #[test]
    fn test_active_runway() {
        let station = Station {
            name: String::from("Kutaisi"),
            atis_freq: 251_000_000,
            traffic_freq: None,
            airfield: Airfield {
                name: String::from("Kutaisi"),
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    alt: 0.0,
                },
                runways: vec![String::from("04"), String::from("22")],
            },
            static_wind: None,
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
            airfield: Airfield {
                name: String::from("Kutaisi"),
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    alt: 0.0,
                },
                runways: vec![String::from("04"), String::from("22")],
            },
            static_wind: None,
            dynamic_weather: DynamicWeather::create("").unwrap(),
        };

        let report = station.generate_report(26).unwrap();
        assert_eq!(report, r"This is Kutaisi information Alpha. Runway in use is 0 4. Wind 3 3 0, at 1 0 knots. Temperature 2 2 celcius, ALTIMETER 2 9 decimal 9 7. Traffic frequency 2 4 9 decimal 5. REMARKS 1 0 hectopascal. End information Alpha. ");
    }
}
