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
mod mission_info;

use std::ffi::CString;
use std::ptr;

use datis_core::Datis;
use hlua51::{Lua, LuaFunction, LuaTable};
use libc::c_int;
use lua51_sys as ffi;

static mut INITIALIZED: bool = false;
static mut DATIS: Option<Datis> = None;

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

        let config = Config::builder()
            .appender(Appender::builder().build("file", Box::new(requests)))
            .logger(Logger::builder().build(
                "datis",
                if is_debug_loglevel {
                    LevelFilter::Debug
                } else {
                    LevelFilter::Info
                },
            ))
            .logger(Logger::builder().build(
                "datis_core",
                if is_debug_loglevel {
                    LevelFilter::Debug
                } else {
                    LevelFilter::Info
                },
            ))
            .logger(Logger::builder().build(
                "srs",
                if is_debug_loglevel {
                    LevelFilter::Debug
                } else {
                    LevelFilter::Info
                },
            ))
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
                Ok(datis)
            }) {
                Ok(datis) => {
                    DATIS = Some(datis);
                }
                Err(err) => {
                    error!("Error initializing DATIS: {}", err.to_string());
                    return report_error(state, &err.to_string());
                }
            }
        }

        if let Err(err) = DATIS.as_mut().unwrap().start() {
            error!("Error starting SRS Client: {}", err.to_string());
            return report_error(state, &err.to_string());
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn stop(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some(datis) = DATIS.take() {
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
        if let Some(ref mut datis) = DATIS {
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
pub extern "C" fn unpause(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some(ref mut datis) = DATIS {
            info!("Resuming ...");
            if let Err(err) = datis.resume() {
                error!("Error resuming SRS Client: {}", err.to_string());
                return report_error(state, &err.to_string());
            }
        }
    }

    0
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
            name: cstr!("unpause"),
            func: Some(unpause),
        },
        ffi::luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];

    ffi::luaL_openlib(state, cstr!("datis"), registration.as_ptr(), 0);

    1
}
