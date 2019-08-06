evm/testdata:
	cd /tmp/ && git clone https://github.com/ethereum/tests jsondata && cd jsondata && git checkout 74cc22b8f

ci:
	cargo fmt --all -- --check
	cargo clippy --all --tests --all-targets -- -D warnings
	cargo test test_vm test_state


TARGET := riscv64-unknown-elf
CC := $(TARGET)-gcc
LD := $(TARGET)-gcc
CFLAGS := -Os -DCKB_NO_MMU -D__riscv_soft_float -D__riscv_float_abi_soft
LDFLAGS := -lm -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections -Wl,-s
CURRENT_DIR := $(shell pwd)
DOCKER_BUILD := docker run -v $(CURRENT_DIR):/src nervos/ckb-riscv-gnu-toolchain:bionic bash -c

riscv/example/raw:
	$(CC) -I./src/riscv/c/ -o ./build/riscv_c_sdk ./examples/riscv_c_sdk.c
	$(CC) -I./src/riscv/c/ -o ./build/riscv_c_fibonacci ./examples/riscv_c_fibonacci.c
	$(CC) -I./src/riscv/c/ -o ./build/riscv_c_simplestorage ./examples/riscv_c_simplestorage.c

riscv/example:
	$(DOCKER_BUILD) "cd /src && make riscv/example/raw"

riscv/tests/raw:
	$(CC) -I./src/riscv/c/ -o ./build/tests/exit_0 ./tests/c/exit_0.c
	$(CC) -I./src/riscv/c/ -o ./build/tests/exit_1 ./tests/c/exit_1.c

riscv/tests:
	$(DOCKER_BUILD) "cd /src && make riscv/tests/raw"

riscv/all: riscv/example \
	riscv/tests

.PHONY: \
	evm/testdata \
	riscv/all \
	ci
