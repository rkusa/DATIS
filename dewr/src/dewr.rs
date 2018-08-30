use std::ffi::{CStr, CString};
use std::sync::mpsc::{self, Receiver, Sender};
use std::{ptr, thread};

use crate::error::{assert_stacksize, LuaError};
use libc::{self, c_char, c_void};
use lua51::{
    luaL_loadbuffer, luaL_openlibs, lua_State, lua_call, lua_getfield, lua_isboolean, lua_isstring,
    lua_newstate, lua_pcall, lua_pop, lua_pushnumber, lua_pushstring, lua_setfield, lua_toboolean,
    lua_tostring, LUA_ERRERR, LUA_ERRMEM, LUA_ERRRUN, LUA_ERRSYNTAX, LUA_GLOBALSINDEX, LUA_MULTRET,
    LUA_OK,
};

static LUA_SOURCE: &str = r#"
    local terrain = require 'terrain'
    isVisible = terrain.isVisible
"#;

pub struct Dewr {
    tx: Sender<()>,
    rx: Receiver<bool>,
}

impl Dewr {
    pub fn create(state: *mut lua_State) -> Result<Self, LuaError> {
        assert_stacksize(state, 1)?;

        let (in_tx, in_rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::channel();

        unsafe {
            if lua_isstring(state, -1) == 0 {
                return Err(LuaError::InvalidArgument(1));
            }

            let cpath = CStr::from_ptr(lua_tostring(state, -1).as_ref().unwrap())
                .to_string_lossy()
                .to_owned();

            lua_pop(state, 1);

            assert_stacksize(state, 0)?;

            thread::spawn(move || {
                let state = match create_lua_state(&cpath) {
                    Ok(state) => state,
                    Err(err) => {
                        // TODO: log instead of panic
                        panic!(format!("{}", err));
                    }
                };

                loop {
                    in_rx.recv().unwrap();
                    match lua_is_visible(state) {
                        Ok(visibility) => {
                            out_tx.send(visibility).unwrap();
                        }
                        Err(err) => {
                            // TODO: log instead of panic
                            panic!(format!("{}", err));
                        }
                    }
                }
            });
        }

        Ok(Dewr {
            tx: in_tx,
            rx: out_rx,
        })
    }

    pub fn is_visible(&self, state: *mut lua_State) -> Result<(), LuaError> {
        assert_stacksize(state, 0)?;

        self.tx.send(()).unwrap();

        Ok(())
    }

    pub fn collect_result(&self, state: *mut lua_State) -> Result<(bool, bool), LuaError> {
        assert_stacksize(state, 0)?;

        if let Some(result) = self.rx.try_recv().ok() {
            Ok((true, result))
        } else {
            Ok((false, false))
        }
    }
}

unsafe fn lua_is_visible(state: *mut lua_State) -> Result<bool, LuaError> {
    assert_stacksize(state, 0)?;

    // move global isVisible function onto the stack
    let is_visible_name = CString::new("isVisible").unwrap();
    lua_getfield(state, LUA_GLOBALSINDEX, is_visible_name.as_ptr());

    // push arguments onto the stack
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

    // call method with 6 arguments and 1 result
    lua_call(state, 6, 1);

    // read result from stack
    if lua_isboolean(state, -1) == 0 {
        return Err(LuaError::InvalidArgument(1));
    }
    let visibility = lua_toboolean(state, -1) == 1;
    lua_pop(state, 1); // cleanup stack

    // make sure we have a clean stack
    assert_stacksize(state, 0)?;

    Ok(visibility)
}

unsafe fn create_lua_state(cpath: &str) -> Result<*mut lua_State, LuaError> {
    // thx to https://github.com/kyren/rlua
    unsafe extern "C" fn allocator(
        _: *mut c_void,
        ptr: *mut c_void,
        _: usize,
        nsize: usize,
    ) -> *mut c_void {
        if nsize == 0 {
            libc::free(ptr);
            std::ptr::null_mut()
        } else {
            libc::realloc(ptr, nsize)
        }
    }

    // create new lua state and load all standard lua libraries
    let state = lua_newstate(Some(allocator), ptr::null_mut());
    luaL_openlibs(state);

    // load global `package` ontp the stack
    let package_name = CString::new("package").unwrap();
    lua_getfield(state, LUA_GLOBALSINDEX, package_name.as_ptr());

    // load new value for `cpath` onto the stack
    let cpath_value = CString::new(cpath).unwrap();
    lua_pushstring(state, cpath_value.as_ptr());

    // set property `cpath` of `package` to new value
    let cpath_name = CString::new("cpath").unwrap();
    lua_setfield(state, -2, cpath_name.as_ptr());

    // cleanup stack (pop `package`)
    lua_pop(state, 1);
    assert_stacksize(state, 0)?;

    // load lua chunk from string onto the stack
    let name = CString::new("init").unwrap();
    match luaL_loadbuffer(
        state,
        LUA_SOURCE.as_ptr() as *const c_char,
        LUA_SOURCE.len(),
        name.as_ptr(),
    ) as u32
    {
        LUA_OK => {}
        LUA_ERRSYNTAX => {
            let err_msg = CStr::from_ptr(lua_tostring(state, -1).as_ref().unwrap())
                .to_string_lossy()
                .to_owned();
            lua_pop(state, 1);
            return Err(LuaError::Custom(format!("Syntax Error: {}", err_msg)));
        }
        LUA_ERRMEM => return Err(LuaError::Custom(format!("memory allocation failed"))),
        _ => unreachable!(),
    }

    // execute lua chunk
    match lua_pcall(state, 0, LUA_MULTRET, 0) as u32 {
        LUA_OK => {}
        LUA_ERRRUN => {
            let err_msg = CStr::from_ptr(lua_tostring(state, -1).as_ref().unwrap())
                .to_string_lossy()
                .to_owned();
            lua_pop(state, 1);
            return Err(LuaError::Custom(format!("Runtime Error: {}", err_msg)));
        }
        LUA_ERRMEM => return Err(LuaError::Custom(format!("memory allocation failed"))),
        LUA_ERRERR => {
            return Err(LuaError::Custom(format!(
                "error while running the error handler"
            )))
        }
        _ => unreachable!(),
    }

    // make sure stack is clean
    assert_stacksize(state, 0)?;

    Ok(state)
}
