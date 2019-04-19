testdata:
	cd /tmp/ && git clone https://github.com/ethereum/tests jsondata && cd jsondata && git checkout 74cc22b8f

ci:
	cargo fmt --all -- --check
	cargo clippy --all --tests --all-targets -- -D warnings
	cd evm && cargo test && cd ..
	cd state && cargo test && cd ..
	RUST_MIN_STACK=134217728 cargo test

.PHONY: testdata ci
