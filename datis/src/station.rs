use lua51::{lua_State, lua_getfield, LUA_GLOBALSINDEX, lua_tonumber, lua_pop, lua_call};
use crate::error::{LuaType, assert_stacksize, assert_type, LuaError};

#[derive(Debug, PartialEq)]
pub struct AtisStation {
    pub name: String,
    pub freq: u64,
    pub airfield: Option<Airfield>,
    pub static_wind: Option<StaticWind>,
}

#[derive(Debug)]
pub struct FinalStation {
    pub name: String,
    pub freq: u64,
    pub airfield: Option<Airfield>,
    pub static_wind: Option<StaticWind>,
    pub state: *mut lua_State,
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
    wind_speed: f64, // in m/s
    wind_dir: f64, // in radians (the direction the wind is coming from)
    temperature: f64, // in °C
    pressure: f64, // in N/m2
}

impl FinalStation {
    pub fn generate_report(&self) -> Result<String, LuaError> {
        // TODO: unwrap
        let weather = unsafe { self.get_current_weather()? };
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

    unsafe fn get_current_weather(&self) -> Result<WeatherInfo, LuaError> {
        let state = self.state;

        assert_stacksize(state, 0)?;

        // get getWeather
        lua_getfield(state, LUA_GLOBALSINDEX, cstr!("getWeather"));
        assert_type(state, LuaType::Function)?;

        // call getWeather, with 0 arguments and 1 result
        lua_call(state, 0, 1);
        assert_stacksize(state, 1)?;
        assert_type(state, LuaType::Table)?;

        // read windSpeed
        lua_getfield(state, -1, cstr!("windSpeed"));
        assert_type(state, LuaType::Number)?;
        let wind_speed = lua_tonumber(state, -1);
        lua_pop(state, 1);

        // read windDir
        lua_getfield(state, -1, cstr!("windDir"));
        assert_type(state, LuaType::Number)?;
        let wind_dir = lua_tonumber(state, -1);
        lua_pop(state, 1);

        // read temp
        lua_getfield(state, -1, cstr!("temp"));
        assert_type(state, LuaType::Number)?;
        let temperature = lua_tonumber(state, -1);
        lua_pop(state, 1);

        // read pressure
        lua_getfield(state, -1, cstr!("pressure"));
        assert_type(state, LuaType::Number)?;
        let pressure = lua_tonumber(state, -1);
        lua_pop(state, 1);

        // pop weather table
        lua_pop(state, 1);
        assert_stacksize(state, 0)?;

        let mut info = WeatherInfo {
            wind_speed, wind_dir, temperature, pressure
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
