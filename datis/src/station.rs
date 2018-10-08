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
    pub fn generate_report(&self) -> Result<String, Error> {
        let weather = self.get_current_weather()?;
        let mut report = format!("This is {}. ", self.name);

        if let Some(rwy) = self.get_active_runway(weather.wind_dir) {
            let rwy = rwy
                .chars()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join(" ");
            report += &format!("Runway is {}. ", rwy);
        } else {
            error!("Could not find active runway for {}", self.name);
        }

        report += &format!(
            "Surface wind {:.0}, {:.0} knots. ",
            weather.wind_dir.to_degrees(),
            weather.wind_speed * 1.94384, // to knots
        );

        report += &format!(
            "Temperature {:.1} degree celcius, QNH {:.0} hectopascal. ",
            weather.temperature,
            weather.pressure / 100.0, // to hPA
        );

        if let Some(traffic_freq) = self.traffic_freq {
            report += &format!("Traffic frequency {}. ", traffic_freq as f64 / 1_000_000.0);
        }

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
            traffic_freq: Some(255_000_000),
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

        let report = station.generate_report().unwrap();
        assert_eq!(report, r"This is Kutaisi. Runway is 0 4. Surface wind 330, 10 knots. Temperature 22.0 degree celcius, QNH 10 hectopascal. Traffic frequency 255. ");
    }
}
