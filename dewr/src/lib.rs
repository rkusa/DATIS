extern crate libc;
extern crate lua51;

mod dewr;
mod error;

use std::ffi::CString;
use std::ptr;

use crate::dewr::Dewr;
use crate::error::LuaError;
use libc::c_int;
use lua51::{luaL_Reg, luaL_openlib, lua_State, lua_pushboolean};

static mut DEWR: Option<Dewr> = None;

#[no_mangle]
pub unsafe extern "C" fn is_visible(state: *mut lua_State) -> c_int {
    let dewr = match DEWR.as_ref() {
        Some(dewr) => dewr,
        None => {
            return LuaError::Uninitialized.report_to(state);
        }
    };

    if let Err(err) = dewr.is_visible(state) {
        return err.report_to(state);
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn collect_result(state: *mut lua_State) -> c_int {
    let dewr = match DEWR.as_ref() {
        Some(dewr) => dewr,
        None => {
            return LuaError::Uninitialized.report_to(state);
        }
    };

    match dewr.collect_result(state) {
        Ok((ok, visiblity)) => {
            lua_pushboolean(state, ok as c_int);
            lua_pushboolean(state, visiblity as c_int);
            2
        }
        Err(err) => err.report_to(state),
    }
}

#[no_mangle]
pub unsafe extern "C" fn init(state: *mut lua_State) -> c_int {
    match Dewr::create(state) {
        Ok(dewr) => {
            DEWR = Some(dewr);
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
pub unsafe extern "C" fn luaopen_dewr(state: *mut lua_State) -> c_int {
    let library_name = CString::new("dewr").unwrap();
    let init_name = CString::new("init").unwrap();
    let is_visible_name = CString::new("is_visible").unwrap();
    let collect_result_name = CString::new("collect_result").unwrap();

    let registration = &[
        luaL_Reg {
            name: init_name.as_ptr(),
            func: Some(init),
        },
        luaL_Reg {
            name: is_visible_name.as_ptr(),
            func: Some(is_visible),
        },
        luaL_Reg {
            name: collect_result_name.as_ptr(),
            func: Some(collect_result),
        },
        luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];

    luaL_openlib(state, library_name.as_ptr(), registration.as_ptr(), 0);

    1
}
