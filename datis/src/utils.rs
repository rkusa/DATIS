use hlua51::{Lua, LuaTable};

pub fn create_lua_state(cpath: &str, code: &str) -> Result<Lua<'static>, ()> {
    let mut lua = Lua::new();
    lua.openlibs();

    {
        // TODO: unwrap
        let mut package: LuaTable<_> = lua.get("package").unwrap();
        package.set("cpath", cpath);
    }

    {
        // TODO: unwrap
        lua.execute::<()>(code).unwrap();
    }

    Ok(lua)
}
