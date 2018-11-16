test:
	@cargo test --all
.PHONY: test

format:
	@rustup component add rustfmt-preview 2> /dev/null
	@cargo fmt
.PHONY: format

format-check:
	@rustup component add rustfmt-preview 2> /dev/null
	@cargo fmt -- --check
.PHONY: format-check

lint:
	@rustup component add clippy-preview 2> /dev/null
	@cargo clippy --tests --all-features -- -D clippy::all
.PHONY: lint
