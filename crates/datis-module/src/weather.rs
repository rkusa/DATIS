use std::sync::{Arc, Mutex};

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
    ) -> Result<Self, anyhow::Error> {
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
}

impl Weather for DcsWeather {
    fn get_at(&self, x: f64, y: f64, alt: f64) -> Result<WeatherInfo, anyhow::Error> {
        let mut inner = self.0.lock().unwrap();
        let clouds = inner.clouds.clone();
        let visibility = inner.visibility;

        // call `getWeather(x, y, alt)`
        let mut get_weather: LuaFunction<_> = get!(inner.lua, "getWeather")?;

        let pressure_qnh: f64 = {
            let mut weather: LuaTable<_> = get_weather
                .call_with_args((x, y, 0))
                .map_err(|_| anyhow!("failed to call lua function getWeather"))?;
            get!(weather, "pressure")
        }?;

        let mut weather: LuaTable<_> = get_weather
            .call_with_args((x, y, alt))
            .map_err(|_| anyhow!("failed to call lua function getWeather"))?;
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

    fn get_for_unit(&self, name: &str) -> Result<Option<WeatherInfo>, anyhow::Error> {
        let (x, y, alt) = {
            let mut inner = self.0.lock().unwrap();

            // call `getUnitPosition(name)`
            let mut get_unit_position: LuaFunction<_> = get!(inner.lua, "getUnitPosition")?;
            let mut pos: LuaTable<_> = get_unit_position
                .call_with_args(name)
                .map_err(|_| anyhow!("failed to call lua function getUnitPosition"))?;
            let x: f64 = get!(pos, "x")?;
            let y: f64 = get!(pos, "y")?;
            let alt: f64 = get!(pos, "alt")?;

            (x, y, alt)
        };

        if x == 0.0 && y == 0.0 && alt == 0.0 {
            Ok(None)
        } else {
            Ok(Some(self.get_at(x, y, alt)?))
        }
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

    getUnitPosition = function(name)
        local unit = Unit.getByName(name)
        if unit == nil then
            return {
                x = 0,
                y = 0,
                alt = 0,
            }
        else
            local pos = unit:getPoint()
            return {
                x = pos.x,
                y = pos.z,
                alt = pos.y,
            }
        end
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

    function getUnitPosition(name)
        return {
            x = 1,
            y = 2,
            alt = 3,
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

    #[test]
    fn test_get_weather_for_unit() {
        let dw = DcsWeather::create("", None, None).unwrap();
        assert_eq!(
            dw.get_for_unit("foobar").unwrap(),
            Some(WeatherInfo {
                clouds: None,
                visibility: None,
                wind_speed: 1.0,
                wind_dir: 294.59155902616465,
                temperature: 3.0,
                pressure_qnh: 42.0,
                pressure_qfe: 42.0,
            })
        );
    }
}
