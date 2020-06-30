#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate const_cstr;
#[macro_use]
extern crate anyhow;

#[macro_use]
mod macros;
mod mission;

use std::ffi::{CStr, CString};
use std::ptr;

use anyhow::Context;
use datis_core::rpc::{MissionRpc, Response};
use datis_core::Datis;
use hlua51::{Lua, LuaFunction, LuaTable};
use libc::c_int;
use lua51_sys as ffi;
use lua51_sys::lua_pop;
use serde_json::Value;

static mut INITIALIZED: bool = false;
static mut DATIS: Option<(Datis, MissionRpc)> = None;

pub fn init(lua: &mut Lua<'_>) -> Result<String, anyhow::Error> {
    // init logging
    use log::LevelFilter;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Logger, Root};

    // read whether debug logging is enabled
    let is_debug_loglevel = {
        // OptionsData.getPlugin("DATIS", "debugLoggingEnabled")
        let mut options_data: LuaTable<_> = get!(lua, "OptionsData")?;
        let mut get_plugin: LuaFunction<_> = get!(options_data, "getPlugin")?;

        let is_debug_loglevel: bool = get_plugin
            .call_with_args(("DATIS", "debugLoggingEnabled"))
            .map_err(|_| anyhow!("failed to read plugin setting debugLoggingEnabled"))?;
        is_debug_loglevel
    };

    let mut lfs: LuaTable<_> = get!(lua, "lfs")?;
    let mut writedir: LuaFunction<_> = get!(lfs, "writedir")?;
    let writedir: String = writedir.call()?;

    if unsafe { !INITIALIZED } {
        let log_file = writedir.clone() + "Logs\\DATIS.log";

        let requests = FileAppender::builder().append(false).build(log_file)?;

        let log_level = if is_debug_loglevel {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };
        let config = Config::builder()
            .appender(Appender::builder().build("file", Box::new(requests)))
            .logger(Logger::builder().build("datis", log_level))
            .logger(Logger::builder().build("datis_core", log_level))
            .logger(Logger::builder().build("srs", log_level))
            .logger(Logger::builder().build("win_tts", log_level))
            .build(Root::builder().appender("file").build(LevelFilter::Off))?;

        log4rs::init_config(config)?;
    }

    unsafe {
        INITIALIZED = true;
    }

    Ok(writedir + "Logs\\")
}

#[no_mangle]
pub extern "C" fn start(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if DATIS.is_none() {
            let mut lua = Lua::from_existing_state(state, false);

            let log_dir = match init(&mut lua) {
                Ok(p) => p,
                Err(err) => {
                    return report_error(state, &err.to_string());
                }
            };

            info!("Starting DATIS version {} ...", env!("CARGO_PKG_VERSION"));

            match mission::extract(lua).and_then(|info| {
                let mut datis = Datis::new(info.stations)?;
                datis.set_port(info.srs_port);
                if !info.gcloud_key.is_empty() {
                    datis.set_gcloud_key(info.gcloud_key);
                }
                if !info.aws_key.is_empty()
                    && !info.aws_secret.is_empty()
                    && !info.aws_region.is_empty()
                {
                    datis.set_aws_keys(info.aws_key, info.aws_secret, info.aws_region);
                }
                datis.set_log_dir(log_dir);
                Ok((datis, info.rpc))
            }) {
                Ok((datis, mission_info)) => {
                    DATIS = Some((datis, mission_info));
                }
                Err(err) => {
                    error!("Error initializing DATIS: {}", err.to_string());
                    return report_error(state, &err.to_string());
                }
            }
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn stop(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some((datis, _)) = DATIS.take() {
            info!("Stopping ...");
            if let Err(err) = datis.stop() {
                error!("Error stopping SRS Client: {}", err.to_string());
                return report_error(state, &err.to_string());
            }
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn pause(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some((ref mut datis, _)) = DATIS {
            info!("Pausing ...");
            if let Err(err) = datis.pause() {
                error!("Error pausing SRS Client: {}", err.to_string());
                return report_error(state, &err.to_string());
            }
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn resume(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some((ref mut datis, _)) = DATIS {
            info!("Resuming ...");
            if let Err(err) = datis.resume() {
                error!("Error resuming SRS Client: {}", err.to_string());
                return report_error(state, &err.to_string());
            }
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn try_next(l: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some((_, ref mut rpc)) = DATIS {
            ffi::lua_gc(l, ffi::LUA_GCSTOP as i32, 0);

            let result =
                match call_try_next(l, rpc).and_then(|r| assert_stack_size(l, 0).map(|_| r)) {
                    Ok(had_next) => {
                        ffi::lua_pushboolean(l, had_next as c_int);
                        1
                    }
                    Err(err) => {
                        error!("Error receiving RPC request: {}", err.to_string());
                        report_error(l, &err.to_string())
                    }
                };

            ffi::lua_gc(l, ffi::LUA_GCRESTART as i32, 0);

            result
        } else {
            0
        }
    }
}

fn report_error(state: *mut ffi::lua_State, msg: &str) -> c_int {
    let msg = CString::new(msg).unwrap();

    unsafe {
        ffi::lua_pushstring(state, msg.as_ptr());
        ffi::lua_error(state);
    }

    0
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn luaopen_datis(state: *mut ffi::lua_State) -> c_int {
    let registration = &[
        ffi::luaL_Reg {
            name: cstr!("start"),
            func: Some(start),
        },
        ffi::luaL_Reg {
            name: cstr!("stop"),
            func: Some(stop),
        },
        ffi::luaL_Reg {
            name: cstr!("pause"),
            func: Some(pause),
        },
        ffi::luaL_Reg {
            name: cstr!("resume"),
            func: Some(resume),
        },
        ffi::luaL_Reg {
            name: const_cstr!("try_next").as_ptr(),
            func: Some(try_next),
        },
        ffi::luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];

    ffi::luaL_openlib(state, cstr!("datis"), registration.as_ptr(), 0);

    1
}

pub unsafe fn call_try_next(
    l: *mut ffi::lua_State,
    rpc: &MissionRpc,
) -> Result<bool, anyhow::Error> {
    // expect 1 argument, ignore other ones
    ffi::lua_settop(l, 1);

    // read callback argument
    if !ffi::lua_isfunction(l, -1) {
        ffi::lua_settop(l, 0);
        return Err(anyhow!(
            "Expected argument `callback` to be of type `function`"
        ));
    }

    if let Some(mut req) = rpc.try_next() {
        let method = req.method();
        push_string(l, method);
        match req.take_params() {
            Some(p) => {
                let p = serde_json::to_string(&p)?;
                push_string(l, p);
            }
            None => ffi::lua_pushnil(l),
        }

        ffi::lua_call(l, 2, 1); // 2 args, 1 result

        if !ffi::lua_istable(l, -1) {
            ffi::lua_settop(l, 0);
            return Err(anyhow!("Expected argument `result` to be of type `table`"));
        }

        // check whether we've received an error
        ffi::lua_getfield(l, -1, const_cstr!("error").as_ptr());
        if ffi::lua_isstring(l, -1) == 1 {
            let error = CStr::from_ptr(ffi::lua_tostring(l, -1))
                .to_str()?
                .to_string();
            req.receive(Response::Error(error));

            ffi::lua_settop(l, 0);
            return Ok(true);
        }

        // pop error
        lua_pop(l, 1);

        // check whether we've received a result
        ffi::lua_getfield(l, -1, const_cstr!("result").as_ptr());
        if ffi::lua_isstring(l, -1) == 1 {
            let res = CStr::from_ptr(ffi::lua_tostring(l, -1))
                .to_str()?
                .to_string();
            let res: Value = serde_json::from_str(&res).context("error deserializing response")?;
            req.receive(Response::Success(res));
        }

        ffi::lua_settop(l, 0);
        return Ok(true);
    }

    ffi::lua_settop(l, 0);
    Ok(false)
}

fn assert_stack_size(l: *mut ffi::lua_State, expected: usize) -> Result<(), anyhow::Error> {
    let curr = unsafe { ffi::lua_gettop(l) } as usize;
    if curr != expected {
        Err(anyhow!(
            "Expected a Lua stack size of {}, got {}",
            expected,
            curr
        ))
    } else {
        Ok(())
    }
}

fn push_string<T: Into<Vec<u8>>>(l: *mut ffi::lua_State, t: T) {
    let cs = CString::new(t).unwrap();
    let ptr = cs.into_raw();

    unsafe {
        ffi::lua_pushstring(l, ptr);

        // retake pointer to free memory
        let _ = CString::from_raw(ptr);
    }
}
