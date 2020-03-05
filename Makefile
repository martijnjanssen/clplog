.PHONY: build
build: target/debug/clplog

.PHONY: release
release: target/release/clplog

.PHONY: target/release/clplog
target/release/clplog:
	cargo build --release

.PHONY: target/debug/clplog
target/debug/clplog:
	cargo build

.PHONY: clean
clean:
	rm -rf target
