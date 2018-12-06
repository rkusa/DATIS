#![feature(try_trait)]
#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate const_cstr;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod macros;
mod datis;
mod error;
mod srs;
mod station;
mod tts;
mod utils;
mod weather;
mod worker;

use std::ffi::CString;
use std::ptr;

use crate::datis::Datis;
use crate::error::Error;
use hlua51::{Lua, LuaFunction, LuaTable};
use libc::c_int;
use lua51_sys as ffi;

static mut INITIALIZED: bool = false;
static mut DATIS: Option<Datis> = None;

pub fn init(lua: &mut Lua<'_>) -> Result<(), Error> {
    unsafe {
        if INITIALIZED {
            return Ok(());
        }
        INITIALIZED = true;
    }

    // init logging
    use log::LevelFilter;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Logger, Root};

    let mut lfs: LuaTable<_> = get!(lua, "lfs")?;
    let mut writedir: LuaFunction<_> = get!(lfs, "writedir")?;
    let writedir: String = writedir.call()?;
    let log_file = writedir + "Logs\\DATIS.log";

    let requests = FileAppender::builder()
        .append(false)
        .build(log_file)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(requests)))
        .logger(Logger::builder().build("datis", LevelFilter::Info))
        .build(Root::builder().appender("file").build(LevelFilter::Off))
        .unwrap();

    log4rs::init_config(config).unwrap();

    Ok(())
}

#[no_mangle]
pub extern "C" fn start(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if DATIS.is_none() {
            let mut lua = Lua::from_existing_state(state, false);

            if let Err(err) = init(&mut lua) {
                return report_error(state, &err.to_string());
            }

            info!("Starting DATIS version {} ...", env!("CARGO_PKG_VERSION"));

            match Datis::create(lua) {
                Ok(mut datis) => {
                    for client in datis.clients.iter_mut() {
                        if let Err(err) = client.start() {
                            return report_error(state, &err.to_string());
                        }
                    }
                    DATIS = Some(datis);
                }
                Err(err) => {
                    return report_error(state, &err.to_string());
                }
            }
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn stop(_state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some(datis) = DATIS.take() {
            info!("Stopping ...");
            for client in datis.clients.into_iter() {
                client.stop()
            }
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn pause(_state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some(ref mut datis) = DATIS {
            debug!("Pausing ...");
            for client in &datis.clients {
                client.pause()
            }
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn unpause(_state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some(ref mut datis) = DATIS {
            debug!("Unpausing ...");
            for client in &datis.clients {
                client.unpause()
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
