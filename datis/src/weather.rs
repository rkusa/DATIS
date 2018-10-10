use std::sync::{Arc, Mutex};

use crate::error::Error;
use hlua51::{Lua, LuaFunction, LuaTable};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct StaticWeather {
    pub wind: Wind,
    pub clouds: Clouds,
    pub visibility: u32,
}

#[derive(Debug, Clone)]
pub struct DynamicWeather(Arc<Mutex<Lua<'static>>>);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum WeatherKind {
    Static,
    Dynamic,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Wind {
    pub dir: f64, // in radians (the direction the wind is coming from)
    pub speed: f64, // in m/s
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Clouds {
    pub base: u32,
    pub density: u32,
    pub thickness: u32,
    pub iprecptns: u32,
}

#[derive(Debug, PartialEq)]
pub struct WeatherInfo {
    pub wind_speed: f64,  // in m/s
    pub wind_dir: f64,    // in radians (the direction the wind is coming from)
    pub temperature: f64, // in °C
    pub pressure: f64,    // in N/m2
}

#[cfg(not(test))]
static LUA_CODE: &str = r#"
    local Weather = require 'Weather'

    getWeather = function(x, y, alt)
        local position = {
            x = x,
            y = alt,
            z = y,
        }
        local wind = Weather.getGroundWindAtPoint({
            position = position
        })
        local temp, pressure = Weather.getTemperatureAndPressureAtPoint({
            position = position
        })

        return {
            windSpeed = wind.v,
            windDir = wind.a,
            temp = temp,
            pressure = pressure,
        }
    end
"#;

#[cfg(test)]
static LUA_CODE: &str = r#"
    function getWeather(x, y, alt)
        return {
            windSpeed = x,
            windDir = y,
            temp = alt,
            pressure = 42,
        }
    end
"#;

impl DynamicWeather {
    pub fn create(cpath: &str) -> Result<Self, Error> {
        let mut lua = Lua::new();
        lua.openlibs();

        {
            let mut package: LuaTable<_> = get!(lua, "package")?;
            package.set("cpath", cpath);
        }

        {
            lua.execute::<()>(LUA_CODE)?;
        }

        Ok(DynamicWeather(Arc::new(Mutex::new(lua))))
    }

    pub fn get_at(&self, x: f64, y: f64, alt: f64) -> Result<WeatherInfo, Error> {
        // call `getWeather(x, y, alt)`
        let mut lua = self.0.lock().unwrap();
        let mut get_weather: LuaFunction<_> = get!(lua, "getWeather")?;
        let mut weather: LuaTable<_> = get_weather.call_with_args((x, y, alt))?;

        let wind_speed: f64 = get!(weather, "windSpeed")?;
        let wind_dir: f64 = get!(weather, "windDir")?;
        let temperature: f64 = get!(weather, "temp")?;
        let pressure: f64 = get!(weather, "pressure")?;

        Ok(WeatherInfo {
            wind_speed,
            wind_dir,
            temperature,
            pressure,
        })
    }
}

impl PartialEq for DynamicWeather {
    fn eq(&self, _other: &DynamicWeather) -> bool {
        true
    }
}

#[cfg(test)]
mod test {
    use super::{DynamicWeather, WeatherInfo};

    #[test]
    fn test_get_weather() {
        let dw = DynamicWeather::create("").unwrap();
        assert_eq!(
            dw.get_at(1.0, 2.0, 3.0).unwrap(),
            WeatherInfo {
                wind_speed: 1.0,
                wind_dir: 2.0,
                temperature: 3.0,
                pressure: 42.0,
            }
        );
    }
}
