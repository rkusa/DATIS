use std::sync::{Arc, Mutex};

use crate::error::Error;
use crate::station::BREAK;
use crate::utils::pronounce_number;
use hlua51::{Lua, LuaFunction, LuaTable};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct StaticWeather {
    pub wind: Wind,
    pub clouds: Clouds,
    pub visibility: u32, // in m
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
    pub dir: f64,   // in radians (the direction the wind is coming from)
    pub speed: f64, // in m/s
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Clouds {
    pub base: u32, // in m
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

impl StaticWeather {
    pub fn get_clouds_report(&self) -> String {
        // convert m to nm
        let visibility = (self.visibility as f64 * 0.000539957).round();
        let mut report = format!("Visibility {}. {}", pronounce_number(visibility), BREAK);

        let density = match self.clouds.density {
            2..=5 => Some("few"),
            6..=7 => Some("scattered"),
            8 => Some("broken"),
            9..=10 => Some("overcast"),
            _ => None,
        };
        if let Some(density) = density {
            // convert m to ft, round to lowest 500ft increment and shortened (e.g. 17500 -> 175)
            let base = (self.clouds.base as f64 * 3.28084).round() as u32;
            let base = (base - (base % 500)) / 100;
            report += &format!("Cloud conditions {} {}", density, pronounce_number(base));
            match self.clouds.iprecptns {
                1 => report += ", rain",
                2 => report += ", rain and thunderstorm",
                _ => {}
            }
            report += &format!(". {}", BREAK);
        }
        report
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
    use super::*;

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

    #[test]
    fn test_clouds_report() {
        fn create_clouds_report(
            base: u32,
            density: u32,
            iprecptns: u32,
            visibility: u32,
        ) -> String {
            StaticWeather {
                wind: Wind {
                    dir: 0.0,
                    speed: 0.0,
                },
                clouds: Clouds {
                    base,
                    density,
                    thickness: 0,
                    iprecptns,
                },
                visibility,
            }
            .get_clouds_report()
        }

        assert_eq!(
            create_clouds_report(8400, 1, 0, 80_000),
            "Visibility 4 3. | "
        );
        assert_eq!(
            create_clouds_report(8400, 2, 0, 80_000),
            "Visibility 4 3. | Cloud conditions few 2 7 5. | "
        );
        assert_eq!(
            create_clouds_report(8400, 2, 0, 80_000),
            "Visibility 4 3. | Cloud conditions few 2 7 5. | "
        );
        assert_eq!(
            create_clouds_report(8500, 6, 1, 80_000),
            "Visibility 4 3. | Cloud conditions scattered 2 7 5, rain. | "
        );
        assert_eq!(
            create_clouds_report(8500, 10, 2, 80_000),
            "Visibility 4 3. | Cloud conditions overcast 2 7 5, rain and thunderstorm. | "
        );
    }
}
