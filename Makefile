phony: build
build: target/debug/clplog

phony: release
release: target/release/clplog

phony: target/release/clplog
target/release/clplog:
	cargo build --release

phony: target/debug/clplog
target/debug/clplog:
	cargo build

phony: clean
clean:
	rm -rf target
