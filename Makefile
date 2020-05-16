build:
	cargo build

test:
	cargo test

release:
	cargo build --release --package datis
	powershell copy target/release/datis.dll mod/Mods/tech/DATIS/bin/

# to link mods folder into DCS
# New-Item -ItemType SymbolicLink -Name DATIS -Value M:\Development\DATIS\mod\Mods\tech\DATIS
# New-Item -ItemType SymbolicLink -Name datis-hook.lua -Value M:\Development\DATIS\mod\Scripts\Hooks\datis-hook.lua