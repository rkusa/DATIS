extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=lua5.1");
    // println!("cargo:rustc-link-search=TODO");

    let bindings = bindgen::Builder::default()
        .header("lua-5.1.5/src/lualib.h")
        .header("lua-5.1.5/src/lauxlib.h")
        .clang_arg("-Ilua-5.1/src")
        .whitelist_type("luaL?_.*")
        .whitelist_function("luaL?_.*")
        .whitelist_var("LUA_.*")
        .ctypes_prefix("libc")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
