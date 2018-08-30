extern crate libc;
extern crate lua51;

use std::{ffi::CString, ptr};

use libc::c_int;
use lua51::{luaL_Reg, luaL_openlib, lua_State, lua_pushboolean};

#[no_mangle]
pub unsafe extern "C" fn is_visible(state: *mut lua_State) -> c_int {
    lua_pushboolean(state, true as c_int);

    1
}

#[no_mangle]
pub unsafe extern "C" fn luaopen_terrain(state: *mut lua_State) -> c_int {
    let library_name = CString::new("terrain").unwrap();
    let is_visible_name = CString::new("isVisible").unwrap();

    let registration = &[
        luaL_Reg {
            name: is_visible_name.as_ptr(),
            func: Some(is_visible),
        },
        luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];

    luaL_openlib(state, library_name.as_ptr(), registration.as_ptr(), 0);

    1
}
