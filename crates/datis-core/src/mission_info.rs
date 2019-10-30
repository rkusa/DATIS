use srs::message::Position;

pub trait MissionInfo {
    fn get_weather_at(&self, x: f64, y: f64, alt: f64) -> Result<WeatherInfo, anyhow::Error>;
    fn get_unit_position(&self, name: &str) -> Result<Option<Position>, anyhow::Error>;
    fn get_unit_heading(&self, name: &str) -> Result<Option<f64>, anyhow::Error>;
    fn get_abs_time(&self) -> Result<f64, anyhow::Error>;

    fn get_mission_hour(&self) -> Result<u16, anyhow::Error> {
        let mut time = self.get_abs_time()?;
        let mut h = 0;

        while time >= 86_400.0 {
            time -= 86_400.0;
            // ignore days
        }

        while time >= 3_600.0 {
            time -= 3_600.0;
            h += 1;
        }

        Ok(h)
    }
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
    pub clouds: Option<Clouds>,
    pub visibility: Option<u32>, // in m
    pub wind_speed: f64,         // in m/s
    pub wind_dir: f64,           // in degrees (the direction the wind is coming from)
    pub temperature: f64,        // in °C
    pub pressure_qnh: f64,       // in N/m2
    pub pressure_qfe: f64,       // in N/m2
}

#[derive(Clone)]
pub struct StaticMissionInfo;

impl MissionInfo for StaticMissionInfo {
    fn get_weather_at(&self, _x: f64, _y: f64, _alt: f64) -> Result<WeatherInfo, anyhow::Error> {
        Ok(WeatherInfo {
            clouds: None,
            visibility: None,
            wind_speed: 5.0,
            wind_dir: (330.0f64).to_radians(),
            temperature: 22.0,
            pressure_qnh: 101_500.0,
            pressure_qfe: 101_500.0,
        })
    }

    fn get_unit_position(&self, _name: &str) -> Result<Option<Position>, anyhow::Error> {
        Ok(None)
    }

    fn get_unit_heading(&self, _name: &str) -> Result<Option<f64>, anyhow::Error> {
        Ok(None)
    }

    fn get_abs_time(&self) -> Result<f64, anyhow::Error> {
        Ok(0.0)
    }
}
