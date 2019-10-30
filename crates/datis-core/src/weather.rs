use srs::message::Position;

pub trait Weather {
    fn get_at(&self, x: f64, y: f64, alt: f64) -> Result<WeatherInfo, anyhow::Error>;
    fn get_for_unit(&self, name: &str) -> Result<Option<WeatherInfo>, anyhow::Error>;
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Clouds {
    pub base: u32, // in m
    pub density: u32,
    pub thickness: u32,
    pub iprecptns: u32,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct WeatherInfo {
    pub position: Position,
    pub clouds: Option<Clouds>,
    pub visibility: Option<u32>, // in m
    pub wind_speed: f64,         // in m/s
    pub wind_dir: f64,           // in degrees (the direction the wind is coming from)
    pub temperature: f64,        // in °C
    pub pressure_qnh: f64,       // in N/m2
    pub pressure_qfe: f64,       // in N/m2
}

#[derive(Clone)]
pub struct StaticWeather;

impl Weather for StaticWeather {
    fn get_at(&self, x: f64, y: f64, alt: f64) -> Result<WeatherInfo, anyhow::Error> {
        Ok(WeatherInfo {
            position: Position { x, y, alt },
            clouds: None,
            visibility: None,
            wind_speed: 5.0,
            wind_dir: (330.0f64).to_radians(),
            temperature: 22.0,
            pressure_qnh: 101_500.0,
            pressure_qfe: 101_500.0,
        })
    }

    fn get_for_unit(&self, _name: &str) -> Result<Option<WeatherInfo>, anyhow::Error> {
        self.get_at(0.0, 0.0, 0.0).map(|w| Some(w))
    }
}
