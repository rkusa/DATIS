use crate::error::Error;
use hlua51::{Lua, LuaTable};

pub fn create_lua_state(cpath: &str, code: &str) -> Result<Lua<'static>, Error> {
    let mut lua = Lua::new();
    lua.openlibs();

    {
        let mut package: LuaTable<_> = lua.get("package")?;
        package.set("cpath", cpath);
    }

    {
        lua.execute::<()>(code)?;
    }

    Ok(lua)
}
