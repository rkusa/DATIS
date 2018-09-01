use std::ffi::CString;
use std::{error, fmt};

use libc::c_int;
use lua51::{lua_State, lua_error, lua_gettop, lua_pushstring, lua_type};

#[derive(Debug)]
pub enum LuaError {
    StackSize {
        expected: c_int,
        received: c_int,
    },
    UnexpectedType {
        expected: i32,
        received: i32,
    },
    InvalidArgument(usize),
    #[allow(unused)]
    Custom(String),
    Uninitialized,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum LuaType {
    Nil = 0,
    Boolean = 1,
    LightUserData = 2,
    Number = 3,
    String = 4,
    Table = 5,
    Function = 6,
    UserData = 7,
    Thread = 8,
}

pub fn assert_stacksize(state: *mut lua_State, expected: c_int) -> Result<(), LuaError> {
    let received = unsafe { lua_gettop(state) };
    if received != expected {
        return Err(LuaError::StackSize { expected, received });
    }

    Ok(())
}

pub fn assert_type(state: *mut lua_State, expected: LuaType) -> Result<(), LuaError> {
    let expected = expected as i32;
    let received = unsafe { lua_type(state, -1) };
    if received != expected {
        return Err(LuaError::UnexpectedType { expected, received });
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
            LuaError::UnexpectedType { expected, received } => {
                write!(f, "Expected type {}, got {}", expected, received)
            }
            LuaError::InvalidArgument(pos) => write!(f, "Invalid argument type at {}", pos),
            LuaError::Custom(ref s) => write!(f, "{}", s),
            LuaError::Uninitialized => write!(f, "DATIS has not been initialized"),
        }
    }
}

impl error::Error for LuaError {
    fn description(&self) -> &str {
        match *self {
            LuaError::StackSize { .. } => "invalid stack size",
            LuaError::UnexpectedType { .. } => "invalid stack size",
            LuaError::InvalidArgument(_) => "invalid argument type",
            LuaError::Custom(_) => "custom error",
            LuaError::Uninitialized => "DATIS has not been initialized",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            _ => None,
        }
    }
}
