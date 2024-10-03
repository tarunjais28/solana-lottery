_list:
	@just --justfile {{justfile()}} --list

# Run llvm-cov to calculate the test coverage
llvm-cov:
	#!/bin/bash
	# We use a separate TARGET_DIR so that it won't mess up 
	# incremental compiles of our project
	# CARGO_TARGET_DIR=target/llvm-cov cargo +nightly llvm-cov --html --output-dir ./coverage/llvm-cov --hide-instantiations --features dev
	export CARGO_TARGET_DIR=./target/llvm-cov

	cargo +nightly llvm-cov clean --workspace
	cargo +nightly llvm-cov --features dev --no-report -p nezha_staking
	cargo +nightly llvm-cov --no-report -p nezha_vrf
	cargo +nightly llvm-cov	report \
		--hide-instantiations \
		--ignore-filename-regex '(target/|program/(francium|nezha-testing))' \
		--output-dir coverage/llvm-cov \
		--html --open
