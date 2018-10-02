use std::str::FromStr;
use std::thread;

use crate::error::{assert_stacksize, assert_type, LuaError, LuaType};
use lua51::{
     lua_State, lua_call, lua_getfield,  lua_isstring,
     lua_next,  lua_pop, lua_pushnil, lua_pushnumber, lua_pushstring,
     lua_tonumber, lua_tostring,
    LUA_GLOBALSINDEX,
};
use regex::Regex;
use crate::station::{AtisStation, FinalStation, Position, Airfield, StaticWind};
use crate::utils::create_lua_state;

pub struct Datis {
    pub stations: Vec<AtisStation>,
}

impl Datis {
    pub unsafe fn create(state: *mut lua_State) -> Result<Self, LuaError> {
        assert_stacksize(state, 1)?;

        if lua_isstring(state, -1) == 0 {
            return Err(LuaError::InvalidArgument(1));
        }

        let cpath = from_cstr!(lua_tostring(state, -1));

        lua_pop(state, 1);

        assert_stacksize(state, 0)?;

        // read DCS.getMissionDescription()
        lua_getfield(state, LUA_GLOBALSINDEX, cstr!("DCS"));
        assert_type(state, LuaType::Table)?;
        lua_getfield(state, -1, cstr!("getMissionDescription"));
        assert_type(state, LuaType::Function)?;
        lua_call(state, 0, 1); // call method with 0 arguments and 1 result
        let mission_situation = from_cstr!(lua_tostring(state, -1));
        lua_pop(state, 2);
        assert_stacksize(state, 0)?;

        debug!("Extracting ATIS stations from Mission Situation");

        let mut stations = extract_atis_stations(&mission_situation);

        // read Terrain.GetTerrainConfig('Airdromes')
        lua_getfield(state, LUA_GLOBALSINDEX, cstr!("Terrain"));
        assert_type(state, LuaType::Table)?;

        lua_getfield(state, -1, cstr!("GetTerrainConfig"));
        assert_type(state, LuaType::Function)?;

        lua_pushstring(state, cstr!("Airdromes"));
        lua_call(state, 1, 1); // call method with 1 arguments and 1 result

        // stack should look like: airdromes table, terrain table
        assert_stacksize(state, 2)?;

        // iterate airdromes
        lua_pushnil(state); // first key
        while lua_next(state, -2) != 0 {
            // stack looks like: -1 value, -2 key, -3 airdrome

            // get airdrome.id
            lua_getfield(state, -1, cstr!("id"));
            assert_type(state, LuaType::String)?;
            let id = from_cstr!(lua_tostring(state, -1));
            lua_pop(state, 1);

            // get airdrome.display_name
            lua_getfield(state, -1, cstr!("display_name"));
            assert_type(state, LuaType::String)?;
            let display_name = from_cstr!(lua_tostring(state, -1));
            lua_pop(state, 1);

            for station in stations.iter_mut() {
                if station.name != id && station.name != display_name {
                    continue;
                }

                station.name = display_name.to_string();

                // get airdrome.reference_point
                lua_getfield(state, -1, cstr!("reference_point"));
                assert_type(state, LuaType::Table)?;

                lua_getfield(state, -1, cstr!("x"));
                assert_type(state, LuaType::Number)?;
                let x = lua_tonumber(state, -1);
                lua_pop(state, 1);

                lua_getfield(state, -1, cstr!("y"));
                assert_type(state, LuaType::Number)?;
                let y = lua_tonumber(state, -1);
                lua_pop(state, 1);

                // pop reference_point
                lua_pop(state, 1);

                // get airdrome.default_camera_position.pnt[2]
                lua_getfield(state, -1, cstr!("default_camera_position"));
                assert_type(state, LuaType::Table)?;
                lua_getfield(state, -1, cstr!("pnt"));
                assert_type(state, LuaType::Table)?;
                lua_pushnumber(state, 1.0); // first key
                lua_next(state, -2);
                assert_type(state, LuaType::Number)?;
                // this is only the alt of the camera position of the airfield, which seems to be
                // usually elevated by about 100
                let alt = lua_tonumber(state, -1) - 100.0;
                lua_pop(state, 4); // pop value, key, pnt, default_camera_position
                assert_stacksize(state, 4)?; // terrain, airdromes, airdrome key, airdrome value

                // iterate runways
                let mut runways: Vec<String> = Vec::new();
                lua_getfield(state, -1, cstr!("runways"));
                lua_pushnil(state); // first key
                while lua_next(state, -2) != 0 {
                    // get runway.start
                    lua_getfield(state, -1, cstr!("start"));
                    assert_type(state, LuaType::String)?;
                    let start = from_cstr!(lua_tostring(state, -1));
                    lua_pop(state, 1);
                    runways.push(start.to_string());

                    // get runway.end
                    lua_getfield(state, -1, cstr!("end"));
                    assert_type(state, LuaType::String)?;
                    let end = from_cstr!(lua_tostring(state, -1));
                    lua_pop(state, 1);
                    runways.push(end.to_string());

                    // remove value from stack so we can continue with the next key
                    lua_pop(state, 1);
                }

                // pop airdrome.runways
                lua_pop(state, 1);

                station.airfield = Some(Airfield {
                    position: Position { x, y, alt },
                    runways,
                });

                break;
            }

            // remove value from stack so we can continue with the next key
            lua_pop(state, 1);
            assert_stacksize(state, 3)?;
        }

        // airdromes table, terrain table (key has been consumed on last lua_next call)
        lua_pop(state, 2);
        assert_stacksize(state, 0)?;

        stations.retain(|s| s.airfield.is_some());

        // get _current_mission.mission.weather
        lua_getfield(state, LUA_GLOBALSINDEX, cstr!("_current_mission"));
        assert_type(state, LuaType::Table)?;
        lua_getfield(state, -1, cstr!("mission"));
        assert_type(state, LuaType::Table)?;
        lua_getfield(state, -1, cstr!("weather"));
        assert_type(state, LuaType::Table)?;

        // get atmosphere_type
        lua_getfield(state, -1, cstr!("atmosphere_type"));
        assert_type(state, LuaType::Number)?;
        let atmosphere_type = lua_tonumber(state, -1);
        lua_pop(state, 1);
        if atmosphere_type == 0.0 { // is static DCS weather system
            // get wind
            lua_getfield(state, -1, cstr!("wind"));
            assert_type(state, LuaType::Table)?;
            // get wind_at_ground
            lua_getfield(state, -1, cstr!("wind"));
            assert_type(state, LuaType::Table)?;

            // get wind_at_ground.speed
            lua_getfield(state, -1, cstr!("speed"));
            assert_type(state, LuaType::Number)?;
            let wind_speed = lua_tonumber(state, -1);
            lua_pop(state, 1);

            // get wind_at_ground.dir
            lua_getfield(state, -1, cstr!("dir"));
            assert_type(state, LuaType::Number)?;
            let mut wind_dir = lua_tonumber(state, -1);
            lua_pop(state, 1);

            for station in stations.iter_mut() {
                // rotate dir
                wind_dir -= 180.0;
                if wind_dir < 0.0 {
                    wind_dir += 360.0;
                }

                station.static_wind = Some(StaticWind{
                    dir: wind_dir.to_radians(), speed: wind_speed
                });
            }

            // pop wind_at_ground, wind
            lua_pop(state, 2);
        }

        // pop weather, mission, _current_mission
        lua_pop(state, 3);
        assert_stacksize(state, 0)?;

        debug!("ATIS Stations:");
        for station in &stations {
            debug!("  - {} (Freq: {})", station.name, station.freq);
        }

        for station in stations {
            let cpath = cpath.clone();
            thread::spawn(move || {
                let airfield = station.airfield.as_ref().unwrap();
                let lua = format!(
                    r#"
                    local Weather = require 'Weather'
                    local position = {{
                        x = {},
                        y = {},
                        z = {},
                    }}

                    getWeather = function()
                        local wind = Weather.getGroundWindAtPoint({{ position = position }})
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
                "#,
                    airfield.position.x,
                    airfield.position.alt,
                    airfield.position.y,
                );
                debug!("Loading Lua: {}", lua);
                let new_state = match create_lua_state(&cpath, &lua) {
                    Ok(state) => state,
                    Err(err) => {
                        error!("{}", err);
                        return;
                    }
                };
                let station = FinalStation {
                     name: station.name,
                     freq: station.freq,
                     airfield: station.airfield,
                     static_wind: station.static_wind,
                     state: new_state,
                };

                if let Err(err) = crate::srs::start(station) {
                    error!("{}", err);
                }
            });
        }

        Ok(Datis { stations: Vec::new() })
    }
}

fn extract_atis_stations(situation: &str) -> Vec<AtisStation> {
    let re = Regex::new(r"ATIS ([a-zA-Z-]+) ([1-3]\d{2}(\.\d{1,3})?)").unwrap();
    re.captures_iter(situation)
        .map(|caps| {
            let name = caps.get(1).unwrap().as_str().to_string();
            let freq = caps.get(2).unwrap().as_str();
            let freq = (f32::from_str(freq).unwrap() * 1_000_000.0) as u64;
            AtisStation {
                name,
                freq,
                airfield: None,
                static_wind: None,
            }
        }).collect()
}

#[cfg(test)]
mod test {
    use super::{extract_atis_stations, AtisStation};

    #[test]
    fn test_atis_extraction() {
        let stations = extract_atis_stations(
            r#"
            ATIS Kutaisi 251.000
            ATIS Batumi 131.5
            ATIS Senaki-Kolkhi 145
        "#,
        );

        assert_eq!(
            stations,
            vec![
                AtisStation {
                    name: "Kutaisi".to_string(),
                    freq: 251_000_000,
                    airfield: None,
                    static_wind: None,
                },
                AtisStation {
                    name: "Batumi".to_string(),
                    freq: 131_500_000,
                    airfield: None,
                    static_wind: None,
                },
                AtisStation {
                    name: "Senaki-Kolkhi".to_string(),
                    freq: 145_000_000,
                    airfield: None,
                    static_wind: None,
                }
            ]
        );
    }
}
