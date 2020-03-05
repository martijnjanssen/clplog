phony: build
build: target/debug/clplog

phony: release
release: target/release/clplog

target/release/clplog:
	cargo build --release

target/debug/clplog:
	cargo build

phony: clean
clean:
	rm -rf target
