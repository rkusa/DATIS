#![warn(rust_2018_idioms)]

mod mission;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use datis_core::rpc::MissionRpc;
use datis_core::Datis;
use mlua::prelude::*;
use mlua::{Function, Value};
use once_cell::sync::Lazy;

static INITIALIZED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static DATIS: Lazy<RwLock<Option<(Datis, MissionRpc)>>> = Lazy::new(|| RwLock::new(None));

pub fn init(lua: &Lua) -> Result<String, mlua::Error> {
    // init logging
    use log::LevelFilter;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Logger, Root};

    let mut write_dir: String = {
        let lfs: LuaTable<'_> = lua.globals().get("lfs")?;
        lfs.call_method("writedir", ())?
    };
    write_dir += "Logs\\";

    if INITIALIZED.swap(true, Ordering::Relaxed) {
        return Ok(write_dir);
    }

    // read whether debug logging is enabled
    let is_debug_loglevel = {
        // OptionsData.getPlugin("DATIS", "debugLoggingEnabled")
        let options_data: LuaTable<'_> = lua.globals().get("OptionsData")?;
        let is_debug_loglevel: bool =
            options_data.call_function("getPlugin", ("DATIS", "debugLoggingEnabled"))?;
        is_debug_loglevel
    };

    let log_file = write_dir.clone() + "DATIS.log";

    let requests = FileAppender::builder().append(false).build(log_file)?;

    let log_level = if is_debug_loglevel {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(requests)))
        .logger(Logger::builder().build("datis", log_level))
        .logger(Logger::builder().build("datis_core", log_level))
        .logger(Logger::builder().build("srs", log_level))
        .logger(Logger::builder().build("win_tts", log_level))
        .logger(Logger::builder().build("dcs_module_rpc", log_level))
        .build(Root::builder().appender("file").build(LevelFilter::Off))
        .map_err(to_lua_err)?;

    log4rs::init_config(config).map_err(to_lua_err)?;

    Ok(write_dir)
}

fn start(lua: &Lua, (): ()) -> LuaResult<()> {
    {
        if DATIS.read().unwrap().is_some() {
            return Ok(());
        }
    }

    let log_dir = init(lua)?;
    log::info!("Starting DATIS version {} ...", env!("CARGO_PKG_VERSION"));

    let start = mission::extract(lua).and_then(|info| {
        let mut datis = Datis::new(info.stations).map_err(to_lua_err)?;
        datis.set_port(info.srs_port);
        if !info.gcloud_key.is_empty() {
            datis.set_gcloud_key(info.gcloud_key);
        }
        if !info.aws_key.is_empty() && !info.aws_secret.is_empty() && !info.aws_region.is_empty() {
            datis.set_aws_keys(info.aws_key, info.aws_secret, info.aws_region);
        }
        datis.set_log_dir(log_dir);
        Ok((datis, info.rpc))
    });
    match start {
        Ok((datis, mission_info)) => {
            let mut d = DATIS.write().unwrap();
            *d = Some((datis, mission_info));
        }
        Err(err) => {
            log::error!("Error initializing DATIS: {}", err.to_string());
            return Err(err);
        }
    }

    Ok(())
}

fn stop(_: &Lua, _: ()) -> LuaResult<()> {
    if let Some((datis, _)) = DATIS.write().unwrap().take() {
        log::info!("Stopping ...");
        if let Err(err) = datis.stop() {
            log::error!("Error stopping SRS Client: {}", err.to_string());
            return Err(to_lua_err(err));
        }
    }

    Ok(())
}

fn pause(_: &Lua, _: ()) -> LuaResult<()> {
    if let Some((ref mut datis, _)) = *DATIS.write().unwrap() {
        log::info!("Pausing ...");
        if let Err(err) = datis.pause() {
            log::error!("Error pausing SRS Client: {}", err.to_string());
            return Err(to_lua_err(err));
        }
    }

    Ok(())
}

fn resume(_: &Lua, _: ()) -> LuaResult<()> {
    if let Some((ref mut datis, _)) = *DATIS.write().unwrap() {
        log::info!("Resuming ...");
        if let Err(err) = datis.resume() {
            log::error!("Error resuming SRS Client: {}", err.to_string());
            return Err(to_lua_err(err));
        }
    }

    Ok(())
}

fn try_next(lua: &Lua, callback: Function<'_>) -> LuaResult<bool> {
    if let Some((_, ref rpc)) = *DATIS.read().unwrap() {
        if let Some(mut next) = rpc.try_next() {
            let method = next.method().to_string();
            let params = next
                .params(lua)
                .map_err(|err| mlua::Error::ExternalError(Arc::new(Error::SerializeParams(err))))?;

            let result: LuaTable<'_> = callback.call((method.as_str(), params))?;
            let error: Option<String> = result.get("error")?;

            if let Some(error) = error {
                next.error(error);
                return Ok(true);
            }

            let res: Value<'_> = result.get("result")?;
            next.success(&res).map_err(|err| {
                mlua::Error::ExternalError(Arc::new(Error::DeserializeResult {
                    err,
                    method,
                    result: pretty_print_value(res, 0)
                        .unwrap_or_else(|err| format!("failed to pretty print result: {}", err)),
                }))
            })?;

            return Ok(true);
        }
    }

    Ok(false)
}

#[mlua_derive::lua_module]
pub fn datis(lua: &Lua) -> LuaResult<LuaTable<'_>> {
    let exports = lua.create_table()?;
    exports.set("start", lua.create_function(start)?)?;
    exports.set("stop", lua.create_function(stop)?)?;
    exports.set("pause", lua.create_function(pause)?)?;
    exports.set("resume", lua.create_function(resume)?)?;
    exports.set("try_next", lua.create_function(try_next)?)?;
    Ok(exports)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to deserialize params: {0}")]
    DeserializeParams(#[source] serde_mlua::Error),
    #[error("Failed to deserialize result for method {method}: {err}\n{result}")]
    DeserializeResult {
        #[source]
        err: serde_mlua::Error,
        method: String,
        result: String,
    },
    #[error("Failed to serialize params: {0}")]
    SerializeParams(#[source] serde_mlua::Error),
}

fn to_lua_err(err: impl std::error::Error + 'static) -> mlua::Error {
    mlua::Error::ExternalError(Arc::new(err))
}

fn pretty_print_value(val: Value<'_>, indent: usize) -> LuaResult<String> {
    Ok(match val {
        Value::Nil => "nil".to_string(),
        Value::Boolean(v) => v.to_string(),
        Value::LightUserData(_) => String::new(),
        Value::Integer(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => format!("\"{}\"", v.to_str()?),
        Value::Table(t) => {
            let mut s = "{\n".to_string();
            for pair in t.pairs::<Value<'_>, Value<'_>>() {
                let (key, value) = pair?;
                s += &format!(
                    "{}{} = {},\n",
                    "  ".repeat(indent + 1),
                    pretty_print_value(key, indent + 1)?,
                    pretty_print_value(value, indent + 1)?
                );
            }
            s += &format!("{}}}", "  ".repeat(indent));
            s
        }
        Value::Function(_) => "[function]".to_string(),
        Value::Thread(_) => String::new(),
        Value::UserData(_) => String::new(),
        Value::Error(err) => err.to_string(),
    })
}
