fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test

nextest:
	cargo nextest run

check: fmt lint test
