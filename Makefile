build:
	cargo build

test:
	cargo test

release:
	cargo build --release
	cp target/release/datis.dll scripts/DATIS/
