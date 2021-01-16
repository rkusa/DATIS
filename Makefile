test:
	cargo test --workspace --exclude datis

test_debug:
	cargo test --workspace --exclude datis -- --nocapture

release: export LUA_LIB_NAME=lua
release: export LUA_LIB=$(CURDIR)/lua5.1/
release: export LUA_INC=$(CURDIR)/lua5.1/include
release:
	cargo build --release --package datis
	powershell copy target/release/datis.dll mod/Mods/services/DATIS/bin/

fmt:
	cargo fmt

clippy: export LUA_LIB_NAME=lua
clippy: export LUA_LIB=$(CURDIR)/lua5.1/
clippy: export LUA_INC=$(CURDIR)/lua5.1/include
clippy:
	cargo clippy

watch: export LUA_LIB_NAME=lua
watch: export LUA_LIB=$(CURDIR)/lua5.1/
watch: export LUA_INC=$(CURDIR)/lua5.1/include
watch:
	cargo watch

# to link mods folder into DCS
# New-Item -ItemType SymbolicLink -Name DATIS -Value M:\Development\DATIS\mod\Mods\services\DATIS
# New-Item -ItemType SymbolicLink -Name datis-hook.lua -Value M:\Development\DATIS\mod\Scripts\Hooks\datis-hook.lua
