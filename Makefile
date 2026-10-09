TARGET ?= aarch64-unknown-linux-gnu
TEST_THREADS ?= 2

.PHONY: build build-target build-aarch64 test fmt

build:
	cargo build --release

build-target:
	cargo build --release --target $(TARGET)

build-aarch64:
	$(MAKE) build-target TARGET=aarch64-unknown-linux-gnu

test:
	cargo test --locked -- --test-threads=$(TEST_THREADS)

fmt:
	cargo fmt --check
