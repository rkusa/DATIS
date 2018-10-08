use std::sync::{Arc, Mutex};

use crate::error::Error;
use hlua51::{Lua, LuaFunction, LuaTable};

#[derive(Debug, Clone)]
pub struct DynamicWeather(Arc<Mutex<Lua<'static>>>);

#[derive(Debug)]
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
        local wind = Weather.getGroundWindAtPoint({{
            position = {{
                x = x,
                y = alt,
                z = y,
            }}
        }})
        local temp, pressure = Weather.getTemperatureAndPressureAtPoint({{
            position = position
        }})

        return {{
            windSpeed = wind.v,
            windDir = wind.a,
            temp = temp,
            pressure = pressure,
        }}
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
            #[cfg(not(test))]
            lua.execute::<()>(LUA_CODE)?;
        }

        Ok(DynamicWeather(Arc::new(Mutex::new(lua))))
    }

    #[cfg(not(test))]
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
