mod config;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use config::read_config;
use datis_core::config::Config;
use datis_core::ipc::MissionRpc;
use datis_core::Datis;
use mlua::prelude::*;
use mlua::{Function, Value};
use once_cell::sync::Lazy;

static INITIALIZED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static DATIS: Lazy<RwLock<Option<(Datis, MissionRpc)>>> = Lazy::new(|| RwLock::new(None));

pub fn init(lua: &Lua) -> Result<(Config, PathBuf), mlua::Error> {
    // init logging
    use log::LevelFilter;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Logger, Root};

    let write_dir: String = {
        let lfs: LuaTable<'_> = lua.globals().get("lfs")?;
        lfs.call_method("writedir", ())?
    };
    let write_dir = PathBuf::from(write_dir);

    let config = read_config(&write_dir).map_err(|err| to_lua_err("reading config file", err))?;

    if INITIALIZED.swap(true, Ordering::Relaxed) {
        return Ok((config, write_dir));
    }

    let log_file = write_dir.join("Logs").join("DATIS.log");
    let requests = FileAppender::builder().append(false).build(log_file)?;

    let log_level = if config.debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    log4rs::init_config(
        Config::builder()
            .appender(Appender::builder().build("file", Box::new(requests)))
            .logger(Logger::builder().build("datis", log_level))
            .logger(Logger::builder().build("datis_core", log_level))
            .logger(Logger::builder().build("srs", log_level))
            .logger(Logger::builder().build("win_tts", log_level))
            .logger(Logger::builder().build("dcs_module_ipc", log_level))
            .build(Root::builder().appender("file").build(LevelFilter::Off))
            .map_err(|err| to_lua_err("creating log config", err))?,
    )
    .map_err(|err| to_lua_err("initializing logging", err))?;

    Ok((config, write_dir))
}

fn start(lua: &Lua, (): ()) -> LuaResult<()> {
    {
        if DATIS.read().unwrap().is_some() {
            return Ok(());
        }
    }

    let (config, write_dir) = init(lua)?;
    log::info!("Starting DATIS version {} ...", env!("CARGO_PKG_VERSION"));
    log::info!("Using SRS Server port: {}", config.srs_port);

    let mut datis = Datis::new(config).map_err(|err| to_lua_err("creating DATIS instance", err))?;
    datis.enable_exporter(write_dir.join("Logs"));

    let mut d = DATIS.write().unwrap();
    *d = Some((datis, MissionRpc::default()));

    Ok(())
}

fn stop(_: &Lua, _: ()) -> LuaResult<()> {
    if let Some((datis, _)) = DATIS.write().unwrap().take() {
        log::info!("Stopping ...");
        if let Err(err) = datis.stop() {
            return Err(to_lua_err("stopping SRS client", err));
        }
    }

    Ok(())
}

fn pause(_: &Lua, _: ()) -> LuaResult<()> {
    if let Some((ref mut datis, _)) = *DATIS.write().unwrap() {
        log::info!("Pausing ...");
        if let Err(err) = datis.pause() {
            return Err(to_lua_err("pausing SRS client", err));
        }
    }

    Ok(())
}

fn resume(_: &Lua, _: ()) -> LuaResult<()> {
    if let Some((ref mut datis, _)) = *DATIS.write().unwrap() {
        log::info!("Resuming ...");
        if let Err(err) = datis.resume() {
            return Err(to_lua_err("resuming SRC client", err));
        }
    }

    Ok(())
}

fn try_next(lua: &Lua, callback: Function<'_>) -> LuaResult<bool> {
    if let Some((_, ref ipc)) = *DATIS.read().unwrap() {
        if let Some(mut next) = ipc.try_next() {
            let method = next.method().to_string();
            let params = next
                .params(lua)
                .map_err(|err| mlua::Error::ExternalError(Arc::new(Error::SerializeParams(err))))?;

            let result: LuaTable<'_> = callback.call((method.as_str(), params))?;
            let error: Option<String> = result.get("error")?;

            if let Some(error) = error {
                next.error(error, None);
                return Ok(true);
            }

            let res = match result.get::<_, Value<'_>>("result") {
                Ok(res) => res,
                Err(_) => {
                    next.error("received empty IPC response".to_string(), None);
                    return Ok(true);
                }
            };

            if let Err(err) = next.success(lua, &res) {
                next.error(
                    format!(
                        "Failed to deserialize result for method {}: {}\n{}",
                        method,
                        err,
                        pretty_print_value(res, 0).unwrap_or_else(|err| format!(
                            "failed to pretty print result: {}",
                            err
                        ))
                    ),
                    None,
                );
            }

            return Ok(true);
        }
    }

    Ok(false)
}

#[mlua::lua_module]
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
    DeserializeParams(#[source] mlua::Error),
    #[error("Failed to serialize params: {0}")]
    SerializeParams(#[source] mlua::Error),
}

fn to_lua_err(context: &str, err: impl std::error::Error + Send + Sync + 'static) -> mlua::Error {
    log::error!("Error {}: {}", context, err.to_string());
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
