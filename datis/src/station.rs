use std::cell::RefCell;

use hlua51::{Lua, LuaFunction, LuaTable};

type LuaError = usize;

#[derive(Debug, PartialEq)]
pub struct AtisStation {
    pub name: String,
    pub freq: u64,
    pub airfield: Option<Airfield>,
    pub static_wind: Option<StaticWind>,
}

#[derive(Debug)]
pub struct FinalStation<'a> {
    pub name: String,
    pub freq: u64,
    pub airfield: Option<Airfield>,
    pub static_wind: Option<StaticWind>,
    pub state: RefCell<Lua<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub alt: f64,
}

#[derive(Debug, PartialEq)]
pub struct Airfield {
    pub position: Position,
    pub runways: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct StaticWind {
    pub dir: f64,
    pub speed: f64,
}

#[derive(Debug)]
struct WeatherInfo {
    wind_speed: f64,  // in m/s
    wind_dir: f64,    // in radians (the direction the wind is coming from)
    temperature: f64, // in °C
    pressure: f64,    // in N/m2
}

impl<'a> FinalStation<'a> {
    pub fn generate_report(&self) -> Result<String, LuaError> {
        // TODO: unwrap
        let weather = self.get_current_weather()?;
        let report = format!(
            r#"
                This is {}.
                Runway is [n].
                Surface wind {}, {:.0} knots.
                Temperature {:.1} degree celcius,
                QNH {:.0} hectopascal.
                Traffic frequency [xxx]
            "#,
            self.name,
            weather.wind_dir,
            weather.wind_speed * 1.94384, // to knots
            weather.temperature,
            weather.pressure / 100.0, // to hPA
        );
        Ok(report)
    }

    fn get_current_weather(&self) -> Result<WeatherInfo, LuaError> {
        let mut lua = self.state.borrow_mut();

        // TODO: unwrap
        let mut get_weather: LuaFunction<_> = lua.get("getWeather").unwrap();

        // TODO: unwrap
        let mut weather: LuaTable<_> = get_weather.call().unwrap();

        // TODO: unwrap
        let wind_speed: f64 = weather.get("windSpeed").unwrap();
        let wind_dir: f64 = weather.get("windDir").unwrap();
        let temperature: f64 = weather.get("temp").unwrap();
        let pressure: f64 = weather.get("pressure").unwrap();

        let mut info = WeatherInfo {
            wind_speed,
            wind_dir,
            temperature,
            pressure,
        };

        if let Some(ref static_wind) = self.static_wind {
            info.wind_speed = static_wind.speed;
            info.wind_dir = static_wind.dir;
        }

        Ok(info)
    }

    pub fn start(&self) -> Result<(), LuaError> {
        let report = self.generate_report()?;
        info!("Report: {}", report);

        Ok(())
    }
}
