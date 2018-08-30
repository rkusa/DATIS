use std::ffi::CString;
use std::{error, fmt};

use libc::c_int;
use lua51::{lua_State, lua_error, lua_gettop, lua_pushstring};

#[derive(Debug)]
pub enum LuaError {
    StackSize {
        expected: c_int,
        received: c_int,
    },
    InvalidArgument(usize),
    #[allow(unused)]
    Custom(String),
    Uninitialized,
}

pub fn assert_stacksize(state: *mut lua_State, expected: c_int) -> Result<(), LuaError> {
    let received = unsafe { lua_gettop(state) };
    if received != expected {
        return Err(LuaError::StackSize { expected, received });
    }

    Ok(())
}

impl LuaError {
    pub fn report_to(&self, state: *mut lua_State) -> c_int {
        let msg = format!("{}", self);
        let msg = CString::new(msg.as_str()).unwrap();

        unsafe {
            lua_pushstring(state, msg.as_ptr());
            lua_error(state);
        }

        0
    }
}

impl fmt::Display for LuaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LuaError::StackSize { expected, received } => {
                write!(f, "Expected stack size of {}, got {}", expected, received)
            }
            LuaError::InvalidArgument(pos) => write!(f, "Invalid argument type at {}", pos),
            LuaError::Custom(ref s) => write!(f, "{}", s),
            LuaError::Uninitialized => write!(f, "DEWR has not been initialized"),
        }
    }
}

impl error::Error for LuaError {
    fn description(&self) -> &str {
        match *self {
            LuaError::StackSize { .. } => "invalid stack size",
            LuaError::InvalidArgument(_) => "invalid argument type",
            LuaError::Custom(_) => "custom error",
            LuaError::Uninitialized => "DEWR has not been initialized",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            _ => None,
        }
    }
}
