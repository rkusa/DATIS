extern crate libc;
extern crate lua51;
extern crate regex;
#[macro_use]
extern crate log;
extern crate simplelog;

#[macro_use]
mod macros;
mod datis;
mod error;

use std::ffi::CString;
use std::fs::File;
use std::ptr;

use crate::datis::Datis;
use crate::error::LuaError;
use libc::c_int;
use lua51::{luaL_Reg, luaL_openlib, lua_State, lua_pushnumber};
use simplelog::*;

static mut DATIS: Option<Datis> = None;

#[no_mangle]
pub unsafe extern "C" fn get_pressure(state: *mut lua_State) -> c_int {
    let datis = match DATIS.as_ref() {
        Some(datis) => datis,
        None => {
            return LuaError::Uninitialized.report_to(state);
        }
    };

    match datis.get_pressure() {
        Ok(pressure) => {
            lua_pushnumber(state, pressure);
            1
        }
        Err(err) => err.report_to(state),
    }
}

#[no_mangle]
pub unsafe extern "C" fn init(state: *mut lua_State) -> c_int {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Debug,
        Config::default(),
        File::create("M:/Saved Games/DCS.openbeta/Logs/DATIS-dll.log").unwrap(),
    )]).unwrap();

    debug!("Initializing ...");

    match Datis::create(state) {
        Ok(datis) => {
            DATIS = Some(datis);
        }
        Err(err) => {
            err.report_to(state);
            return 0;
        }
    }

    0
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn luaopen_datis(state: *mut lua_State) -> c_int {
    let library_name = CString::new("datis").unwrap();
    let init_name = CString::new("init").unwrap();
    let get_pressure_name = CString::new("getPressure").unwrap();

    let registration = &[
        luaL_Reg {
            name: init_name.as_ptr(),
            func: Some(init),
        },
        luaL_Reg {
            name: get_pressure_name.as_ptr(),
            func: Some(get_pressure),
        },
        luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];

    luaL_openlib(state, library_name.as_ptr(), registration.as_ptr(), 0);

    1
}
