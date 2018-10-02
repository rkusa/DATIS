extern crate libc;
extern crate lua51;
extern crate regex;
#[macro_use]
extern crate log;
extern crate simplelog;
#[macro_use] extern crate const_cstr;
extern crate byteorder;
extern crate uuid;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate ogg;
extern crate base64;
extern crate reqwest;


#[macro_use]
mod macros;
mod datis;
mod error;
mod station;
mod utils;
mod srs;

use std::fs::File;
use std::ptr;

use crate::datis::Datis;
use libc::c_int;
use lua51::{luaL_Reg, luaL_openlib, lua_State};
use simplelog::*;

static mut DATIS: Option<Datis> = None;

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
//            for station in &datis.stations {
//                station.start();
//            }
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
    let registration = &[
        luaL_Reg {
            name: cstr!("init"),
            func: Some(init),
        },
        luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];

    luaL_openlib(state, cstr!("datis"), registration.as_ptr(), 0);

    1
}
