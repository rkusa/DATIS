extern crate libc;
extern crate lua51;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::{
    ffi::{CStr, CString},
    os::raw::c_void,
    ptr, thread,
};

use libc::{c_char, c_int};
use lua51::{
    luaL_Reg, luaL_loadbuffer, luaL_openlib, luaL_openlibs, lua_State, lua_call, lua_getfield,
    lua_newstate, lua_pcall, lua_pop, lua_pushboolean, lua_pushnumber, lua_pushstring,
    lua_setfield, lua_toboolean, lua_tolstring, lua_tostring, LUA_GLOBALSINDEX, LUA_MULTRET,
};

const LUA_OK: i32 = 0;

static LUA_SOURCE: &str = r#"
    local terrain = require 'terrain'
    isVisible = terrain.isVisible
"#;

static mut SENDER: Option<Sender<()>> = None;
static mut RECEIVER: Option<Receiver<bool>> = None;

unsafe fn create_lua_state(cpath: &str) -> *mut lua_State {
    // thx to https://github.com/kyren/rlua
    unsafe extern "C" fn allocator(
        _: *mut c_void,
        ptr: *mut c_void,
        _: usize,
        nsize: usize,
    ) -> *mut c_void {
        if nsize == 0 {
            libc::free(ptr as *mut libc::c_void);
            ptr::null_mut()
        } else {
            let p = libc::realloc(ptr as *mut libc::c_void, nsize);
            if p.is_null() {
                // We require that OOM results in an abort, and that the lua allocator function
                // never errors.  Since this is what rust itself normally does on OOM, this is
                // not really a huge loss.  Importantly, this allows us to turn off the gc, and
                // then know that calling Lua API functions marked as 'm' will not result in a
                // 'longjmp' error while the gc is off.
                panic!("out of memory in Lua allocation, aborting!");
            } else {
                p as *mut c_void
            }
        }
    }

    let state = lua_newstate(Some(allocator), ptr::null_mut());
    luaL_openlibs(state);

    let package_name = CString::new("package").unwrap();
    lua_getfield(state, LUA_GLOBALSINDEX, package_name.as_ptr());

    let cpath_value = CString::new(cpath).unwrap();
    lua_pushstring(state, cpath_value.as_ptr());

    let cpath_name = CString::new("cpath").unwrap();
    lua_setfield(state, -2, cpath_name.as_ptr());

    // pop package
    lua_pop(state, 1);

    let name = CString::new("init").unwrap();
    match luaL_loadbuffer(
        state,
        LUA_SOURCE.as_ptr() as *const c_char,
        LUA_SOURCE.len(),
        name.as_ptr(),
    ) {
        LUA_OK => {
            //Ok(Function(self.pop_ref()))
        }
        _err => panic!("LUA ERROR"),
    }

    match lua_pcall(state, 0, LUA_MULTRET, 0) {
        LUA_OK => {
            //Ok(Function(self.pop_ref()))
        }
        err => {
            if let Some(s) = lua_tolstring(state, -1, ptr::null_mut()).as_ref() {
                let err_msg = CStr::from_ptr(s).to_string_lossy().into_owned();
                panic!(err_msg);
            }
            panic!(format!("LUA ERROR 2 {}", err));
        }
    }

    state
}

unsafe fn lua_is_visible(state: *mut lua_State) -> bool {
    let is_visible_name = CString::new("isVisible").unwrap();
    lua_getfield(state, LUA_GLOBALSINDEX, is_visible_name.as_ptr());

    let x1 = 1.0;
    let alt1 = 100.0;
    let y1 = 1.0;

    let x2 = x1;
    let alt2 = alt1;
    let y2 = y1;

    lua_pushnumber(state, x1);
    lua_pushnumber(state, alt1);
    lua_pushnumber(state, y1);
    lua_pushnumber(state, x2);
    lua_pushnumber(state, alt2);
    lua_pushnumber(state, y2);

    lua_call(state, 6, 1);

    lua_toboolean(state, -1) == 1
}

#[no_mangle]
pub unsafe extern "C" fn is_visible(_state: *mut lua_State) -> c_int {
    SENDER.as_ref().unwrap().send(()).unwrap();

    0
}

#[no_mangle]
pub unsafe extern "C" fn collect_result(state: *mut lua_State) -> c_int {
    if let Some(result) = RECEIVER.as_ref().unwrap().try_recv().ok() {
        lua_pushboolean(state, true as c_int);
        lua_pushboolean(state, result as c_int);
    } else {
        lua_pushboolean(state, false as c_int);
        lua_pushboolean(state, false as c_int);
    }

    2
}

#[no_mangle]
pub unsafe extern "C" fn init(state: *mut lua_State) -> c_int {
    // Throw if argument is missing

    let cpath = if let Some(s) = lua_tostring(state, -1).as_ref() {
        CStr::from_ptr(s).to_string_lossy().to_owned()
    } else {
        // TODO
        unimplemented!();
    };

    lua_pop(state, 1);

    let (in_tx, in_rx) = mpsc::channel();
    let (out_tx, out_rx) = mpsc::channel();

    SENDER = Some(in_tx);
    RECEIVER = Some(out_rx);

    thread::spawn(move || {
        let state = create_lua_state(&cpath);

        loop {
            in_rx.recv().unwrap();
            let visibility = lua_is_visible(state);
            out_tx.send(visibility).unwrap();
        }
    });

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
