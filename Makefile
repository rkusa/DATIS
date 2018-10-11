build:
	cargo build

test: build
	lua-5.1 test.lua

release:
	cargo build --release
	cp target/release/datis.dll scripts/DATIS/
