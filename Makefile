test:
	cargo test --workspace --exclude datis

test_debug:
	cargo test --workspace --exclude datis -- --nocapture

release:
	cargo build --release --package datis
	powershell copy target/release/datis.dll mod/Mods/services/DATIS/bin/

fmt:
	cargo fmt

clippy:
	cargo clippy

watch:
	cargo watch

# to link mods folder into DCS
# New-Item -ItemType SymbolicLink -Name DATIS -Value M:\Development\DATIS\mod\Mods\services\DATIS
# New-Item -ItemType SymbolicLink -Name datis-hook.lua -Value M:\Development\DATIS\mod\Scripts\Hooks\datis-hook.lua
