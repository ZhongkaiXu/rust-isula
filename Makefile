TARGET ?= aarch64-unknown-linux-gnu
TEST_THREADS ?= 2
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER ?= aarch64-linux-gnu-gcc
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER

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
