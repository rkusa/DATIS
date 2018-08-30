#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate libc;

use std::ptr;

use libc::{c_char, c_int};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub const LUA_OK: i32 = 0;

pub unsafe extern "C" fn lua_tostring(L: *mut lua_State, idx: c_int) -> *const c_char {
    lua_tolstring(L, idx, ptr::null_mut())
}

pub unsafe extern "C" fn lua_pop(L: *mut lua_State, n: c_int) {
    lua_settop(L, -(n) - 1);
}
