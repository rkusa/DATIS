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
mod srs;
mod station;
mod utils;

use std::ffi::CString;
use std::fs::File;
use std::ptr;

use crate::datis::Datis;
use hlua51::Lua;
use libc::c_int;
use lua51_sys as ffi;
use simplelog::*;

static mut DATIS: Option<Datis> = None;

#[no_mangle]
pub extern "C" fn init(state: *mut ffi::lua_State) -> c_int {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Debug,
        Config::default(),
        // TODO: detect correct path
        File::create("M:/Saved Games/DCS.openbeta/Logs/DATIS-dll.log").unwrap(),
    )])
    .unwrap();

    debug!("Initializing ...");

    unsafe {
        let received = ffi::lua_gettop(state);
        if received != 1 {
            // expect 1 argument
            return report_error(state, "expected 1 argument: cpath");
        }

        if ffi::lua_isstring(state, -1) == 0 {
            ffi::lua_pop(state, 1);
            return report_error(state, "argument cpath must be a string");
        }

        let lua = Lua::from_existing_state(state, false);
        let cpath = from_cstr!(ffi::lua_tostring(state, -1));
        ffi::lua_pop(state, 1); // remove argument from stack

        match Datis::create(lua, cpath.into_owned()) {
            Ok(datis) => {
                //            for station in &datis.stations {
                //                station.start();
                //            }
                DATIS = Some(datis);
            }
            Err(_err) => {
                //            err.report_to(state);
                return 0;
            }
        }

        0
    }
}

unsafe fn report_error(state: *mut ffi::lua_State, msg: &str) -> c_int {
    let msg = CString::new(msg).unwrap();

    ffi::lua_pushstring(state, msg.as_ptr());
    ffi::lua_error(state);

    0
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn luaopen_datis(state: *mut ffi::lua_State) -> c_int {
    let registration = &[
        ffi::luaL_Reg {
            name: cstr!("init"),
            func: Some(init),
        },
        ffi::luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];

    ffi::luaL_openlib(state, cstr!("datis"), registration.as_ptr(), 0);

    1
}
