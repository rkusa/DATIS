use crate::station::Position;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Clouds {
    pub base: u32, // in m
    pub density: u32,
    pub thickness: u32,
    pub iprecptns: u32,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct WeatherInfo {
    pub clouds: Option<Clouds>,
    pub visibility: Option<u32>, // in m
    pub wind_speed: f64,         // in m/s
    pub wind_dir: f64,           // in degrees (the direction the wind is coming from)
    pub temperature: f64,        // in °C
    pub pressure_qnh: f64,       // in N/m2
    pub pressure_qfe: f64,       // in N/m2
    pub position: Position,
}
