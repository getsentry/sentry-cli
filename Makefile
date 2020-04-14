test:
	@cargo test --all
.PHONY: test

format:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt
.PHONY: format

format-check:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt -- --check
.PHONY: format-check

lint:
	@rustup component add clippy 2> /dev/null
	@cargo clippy --tests --all-features -- -D clippy::all
.PHONY: lint

check: lint format-check
.PHONY: check
