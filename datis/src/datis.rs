use std::ffi::{CStr, CString};
//use std::sync::mpsc::{self, Receiver, Sender};
use std::ptr;
use std::str::FromStr;

use crate::error::{assert_stacksize, assert_type, LuaError, LuaType};
use libc::{self, c_char, c_void};
use lua51::{
    luaL_loadbuffer, luaL_openlibs, lua_State, lua_call, lua_getfield, lua_isnumber, lua_isstring,
    lua_newstate, lua_next, lua_pcall, lua_pop, lua_pushnil, lua_pushnumber, lua_pushstring,
    lua_setfield, lua_tonumber, lua_tostring, LUA_ERRERR, LUA_ERRMEM, LUA_ERRRUN, LUA_ERRSYNTAX,
    LUA_GLOBALSINDEX, LUA_MULTRET, LUA_OK,
};
use regex::Regex;

static LUA_SOURCE: &str = include_str!("lua_env.lua");

pub struct Datis {
    state: *mut lua_State,
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

        debug!("ATIS Stations:");
        for station in &stations {
            debug!("  - {} (Freq: {})", station.name, station.freq);
        }

        let new_state = create_lua_state(&cpath)?;

        Ok(Datis { state: new_state })
    }

    pub unsafe fn get_pressure(&self) -> Result<f64, LuaError> {
        assert_stacksize(self.state, 0)?;

        // move global isVisible function onto the stack
        let get_pressure_name = CString::new("getPressure").unwrap();
        lua_getfield(self.state, LUA_GLOBALSINDEX, get_pressure_name.as_ptr());

        // call method with 0 arguments and 1 result
        lua_call(self.state, 0, 1);

        // read result from stack
        if lua_isnumber(self.state, -1) == 0 {
            return Err(LuaError::InvalidArgument(1));
        }
        let pressure = lua_tonumber(self.state, -1);
        lua_pop(self.state, 1); // cleanup stack

        // make sure we have a clean stack
        assert_stacksize(self.state, 0)?;

        Ok(pressure)
    }
}

unsafe fn create_lua_state(cpath: &str) -> Result<*mut lua_State, LuaError> {
    // thx to https://github.com/kyren/rlua
    unsafe extern "C" fn allocator(
        _: *mut c_void,
        ptr: *mut c_void,
        _: usize,
        nsize: usize,
    ) -> *mut c_void {
        if nsize == 0 {
            libc::free(ptr);
            std::ptr::null_mut()
        } else {
            libc::realloc(ptr, nsize)
        }
    }

    // create new lua state and load all standard lua libraries
    let state = lua_newstate(Some(allocator), ptr::null_mut());
    luaL_openlibs(state);

    // load global `package` onto the stack
    let package_name = CString::new("package").unwrap();
    lua_getfield(state, LUA_GLOBALSINDEX, package_name.as_ptr());

    // load new value for `cpath` onto the stack
    let cpath_value = CString::new(cpath).unwrap();
    lua_pushstring(state, cpath_value.as_ptr());

    // set property `cpath` of `package` to new value
    let cpath_name = CString::new("cpath").unwrap();
    lua_setfield(state, -2, cpath_name.as_ptr());

    // cleanup stack (pop `package`)
    lua_pop(state, 1);
    assert_stacksize(state, 0)?;

    // load lua chunk from string onto the stack
    let name = CString::new("init").unwrap();
    match luaL_loadbuffer(
        state,
        LUA_SOURCE.as_ptr() as *const c_char,
        LUA_SOURCE.len(),
        name.as_ptr(),
    ) as u32
    {
        LUA_OK => {}
        LUA_ERRSYNTAX => {
            let err_msg = CStr::from_ptr(lua_tostring(state, -1).as_ref().unwrap())
                .to_string_lossy()
                .to_owned();
            lua_pop(state, 1);
            return Err(LuaError::Custom(format!("Syntax Error: {}", err_msg)));
        }
        LUA_ERRMEM => return Err(LuaError::Custom(format!("memory allocation failed"))),
        _ => unreachable!(),
    }

    // execute lua chunk
    match lua_pcall(state, 0, LUA_MULTRET, 0) as u32 {
        LUA_OK => {}
        LUA_ERRRUN => {
            let err_msg = CStr::from_ptr(lua_tostring(state, -1).as_ref().unwrap())
                .to_string_lossy()
                .to_owned();
            lua_pop(state, 1);
            return Err(LuaError::Custom(format!("Runtime Error: {}", err_msg)));
        }
        LUA_ERRMEM => return Err(LuaError::Custom(format!("memory allocation failed"))),
        LUA_ERRERR => {
            return Err(LuaError::Custom(format!(
                "error while running the error handler"
            )))
        }
        _ => unreachable!(),
    }

    // make sure stack is clean
    assert_stacksize(state, 0)?;

    Ok(state)
}

#[derive(Debug, PartialEq)]
struct AtisStation {
    name: String,
    freq: u64,
    airfield: Option<Airfield>,
}

#[derive(Debug, PartialEq)]
struct Position {
    x: f64,
    y: f64,
    alt: f64,
}

#[derive(Debug, PartialEq)]
struct Airfield {
    position: Position,
    runways: Vec<String>,
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
                },
                AtisStation {
                    name: "Batumi".to_string(),
                    freq: 131_500_000,
                    airfield: None,
                },
                AtisStation {
                    name: "Senaki-Kolkhi".to_string(),
                    freq: 145_000_000,
                    airfield: None,
                }
            ]
        );
    }
}
