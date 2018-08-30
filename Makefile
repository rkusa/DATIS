build:
	cargo build

test: build
	LD_LIBRARY_PATH=$(shell rustc --print sysroot)/lib:$$LD_LIBRARY_PATH lua-5.1 test.lua