use std::ffi::CString;

use libc::{self, c_void, c_char};
use lua51::{
    luaL_loadbuffer, luaL_openlibs, lua_State, lua_getfield,
    lua_newstate,  lua_pcall, lua_pop, lua_pushstring,
    lua_setfield, lua_tostring, LUA_ERRERR, LUA_ERRMEM, LUA_ERRRUN, LUA_ERRSYNTAX,
    LUA_GLOBALSINDEX, LUA_MULTRET, LUA_OK,
};
use crate::error::{LuaError, assert_stacksize};

pub unsafe fn create_lua_state(cpath: &str, lua: &str) -> Result<*mut lua_State, LuaError> {
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
    let state = lua_newstate(Some(allocator), std::ptr::null_mut());
    luaL_openlibs(state);

    // load global `package` onto the stack
    lua_getfield(state, LUA_GLOBALSINDEX, cstr!("package"));

    // load new value for `cpath` onto the stack
    let cpath = CString::new(cpath).unwrap();
    lua_pushstring(state, cpath.as_ptr());

    // set property `cpath` of `package` to new value
    lua_setfield(state, -2, cstr!("cpath"));

    // cleanup stack (pop `package`)
    lua_pop(state, 1);
    assert_stacksize(state, 0)?;

    // load lua chunk from string onto the stack
    match luaL_loadbuffer(
        state,
        lua.as_ptr() as *const c_char,
        lua.len(),
        cstr!("init"),
    ) as u32
        {
            LUA_OK => {}
            LUA_ERRSYNTAX => {
                let err_msg = from_cstr!(lua_tostring(state, -1));
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
            let err_msg = from_cstr!(lua_tostring(state, -1));
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
