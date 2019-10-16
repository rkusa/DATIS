use std::error;
use std::sync::{Arc, Mutex};

use crate::error::Error;
use datis_core::weather::{Clouds, Weather, WeatherInfo};
use hlua51::{Lua, LuaFunction, LuaTable};

#[derive(Debug)]
pub struct DcsWeatherInner {
    lua: Lua<'static>,
    clouds: Option<Clouds>,
    visibility: Option<u32>, // in m
}

#[derive(Debug, Clone)]
pub struct DcsWeather(Arc<Mutex<DcsWeatherInner>>);

impl DcsWeather {
    pub fn create(
        cpath: &str,
        clouds: Option<Clouds>,
        visibility: Option<u32>,
    ) -> Result<Self, Error> {
        let mut lua = Lua::new();
        lua.openlibs();

        {
            let mut package: LuaTable<_> = get!(lua, "package")?;
            package.set("cpath", cpath);
        }

        {
            lua.execute::<()>(LUA_CODE)?;
        }

        Ok(DcsWeather(Arc::new(Mutex::new(DcsWeatherInner {
            lua,
            clouds,
            visibility,
        }))))
    }

    fn get_at(&self, x: f64, y: f64, alt: f64) -> Result<WeatherInfo, Error> {
        let mut inner = self.0.lock().unwrap();
        let clouds = inner.clouds.clone();
        let visibility = inner.visibility;

        // call `getWeather(x, y, alt)`
        let mut get_weather: LuaFunction<_> = get!(inner.lua, "getWeather")?;

        let pressure_qnh: f64 = {
            let mut weather: LuaTable<_> = get_weather.call_with_args((x, y, 0))?;
            get!(weather, "pressure")
        }?;

        let mut weather: LuaTable<_> = get_weather.call_with_args((x, y, alt))?;
        let wind_speed: f64 = get!(weather, "windSpeed")?;
        let mut wind_dir: f64 = get!(weather, "windDir")?; // in knots
        let temperature: f64 = get!(weather, "temp")?;
        let pressure_qfe: f64 = get!(weather, "pressure")?;

        // convert to degrees and rotate wind direction
        wind_dir = wind_dir.to_degrees() - 180.0;

        // normalize wind direction
        while wind_dir < 0.0 {
            wind_dir += 360.0;
        }

        Ok(WeatherInfo {
            clouds,
            visibility,
            wind_speed,
            wind_dir,
            temperature,
            pressure_qnh,
            pressure_qfe,
        })
    }
}

impl Weather for DcsWeather {
    fn get_at(&self, x: f64, y: f64, alt: f64) -> Result<WeatherInfo, Box<dyn error::Error>> {
        let info = DcsWeather::get_at(self, x, y, alt)?;
        Ok(info)
    }
}

impl PartialEq for DcsWeather {
    fn eq(&self, other: &DcsWeather) -> bool {
        let lhs = self.0.lock().unwrap();
        let rhs = other.0.lock().unwrap();
        lhs.clouds == rhs.clouds && lhs.visibility == rhs.visibility
    }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_weather() {
        let dw = DcsWeather::create("", None, None).unwrap();
        assert_eq!(
            dw.get_at(1.0, 2.0_f64.to_radians(), 3.0).unwrap(),
            WeatherInfo {
                clouds: None,
                visibility: None,
                wind_speed: 1.0,
                wind_dir: 182.0,
                temperature: 3.0,
                pressure_qnh: 42.0,
                pressure_qfe: 42.0,
            }
        );
    }
}
