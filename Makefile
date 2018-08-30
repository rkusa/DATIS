build:
	cargo build

test: build
	lua-5.1 test.lua
