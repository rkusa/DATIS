extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {
    cc::Build::new()
        .file("lua-5.1.5/src/lapi.c")
        .file("lua-5.1.5/src/lauxlib.c")
        .file("lua-5.1.5/src/lbaselib.c")
        .file("lua-5.1.5/src/lcode.c")
        .file("lua-5.1.5/src/ldblib.c")
        .file("lua-5.1.5/src/ldebug.c")
        .file("lua-5.1.5/src/ldo.c")
        .file("lua-5.1.5/src/ldump.c")
        .file("lua-5.1.5/src/lfunc.c")
        .file("lua-5.1.5/src/lgc.c")
        .file("lua-5.1.5/src/linit.c")
        .file("lua-5.1.5/src/liolib.c")
        .file("lua-5.1.5/src/llex.c")
        .file("lua-5.1.5/src/lmathlib.c")
        .file("lua-5.1.5/src/lmem.c")
        .file("lua-5.1.5/src/loadlib.c")
        .file("lua-5.1.5/src/lobject.c")
        .file("lua-5.1.5/src/lopcodes.c")
        .file("lua-5.1.5/src/loslib.c")
        .file("lua-5.1.5/src/lparser.c")
        .file("lua-5.1.5/src/lstate.c")
        .file("lua-5.1.5/src/lstring.c")
        .file("lua-5.1.5/src/lstrlib.c")
        .file("lua-5.1.5/src/ltable.c")
        .file("lua-5.1.5/src/ltablib.c")
        .file("lua-5.1.5/src/ltm.c")
        .file("lua-5.1.5/src/lua.c")
        .file("lua-5.1.5/src/luac.c")
        .file("lua-5.1.5/src/lundump.c")
        .file("lua-5.1.5/src/lvm.c")
        .file("lua-5.1.5/src/lzio.c")
        .file("lua-5.1.5/src/print.c")
        .flag_if_supported("-Wno-deprecated")
        .include("lua-5.1.5/src")
        .compile("liblua.a");

    let bindings = bindgen::Builder::default()
        .header("lua-5.1.5/src/lualib.h")
        .header("lua-5.1.5/src/lauxlib.h")
        .clang_arg("-Ilua-5.1/src")
        .whitelist_type("luaL?_.*")
        .whitelist_function("luaL?_.*")
        .whitelist_var("LUA_.*")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
