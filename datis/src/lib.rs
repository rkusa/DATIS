#![feature(try_trait)]
#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate const_cstr;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod macros;
mod datis;
mod error;
mod srs;
mod station;
mod tts;
mod weather;
mod worker;

use std::ffi::CString;
use std::fs::File;
use std::ptr;

use crate::datis::Datis;
use crate::error::Error;
use hlua51::{Lua, LuaFunction, LuaTable};
use libc::c_int;
use lua51_sys as ffi;
use simplelog::*;

static mut DATIS: Option<Datis> = None;

pub fn init(lua: &mut Lua<'_>) -> Result<(), Error> {
    let mut lfs: LuaTable<_> = get!(lua, "lfs")?;
    let mut writedir: LuaFunction<_> = get!(lfs, "writedir")?;
    let writedir: String = writedir.call()?;
    let log_file = writedir + "Logs\\DATIS-dll.log";

    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Debug,
        Config::default(),
        // TODO: unwrap
        File::create(&log_file).unwrap(),
    )])
    .unwrap();

    Ok(())
}

#[no_mangle]
pub extern "C" fn start(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        if let Some(ref mut datis) = DATIS {
            for client in datis.clients.iter_mut() {
                if let Err(err) = client.start() {
                    return report_error(state, &err.to_string());
                }
            }
        } else {
            let mut lua = Lua::from_existing_state(state, false);

            if let Err(err) = init(&mut lua) {
                return report_error(state, &err.to_string());
            }

            debug!("Initializing ...");

            match Datis::create(lua) {
                Ok(datis) => DATIS = Some(datis),
                Err(err) => {
                    return report_error(state, &err.to_string());
                }
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
            name: ptr::null(),
            func: None,
        },
    ];

    ffi::luaL_openlib(state, cstr!("datis"), registration.as_ptr(), 0);

    1
}
