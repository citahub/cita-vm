testdata:
	cd /tmp/ && git clone https://github.com/ethereum/tests jsondata && cd jsondata && git checkout 74cc22b8f

ci:
	cargo fmt --all -- --check
	cargo clippy --all --tests --all-targets -- -D warnings
	cd evm && cargo test && cd ..
	cargo test

.PHONY: testdata ci
