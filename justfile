list:
	just -l

# Tests

service-tests *args: build-contract
	#!/bin/bash
	set -euxo pipefail
	export SBF_OUT_DIR=$(realpath ./program/target/sbf-solana-solana/release/)
	CONTRACT_FILE=$SBF_OUT_DIR/nezha_staking.so
	test -f $CONTRACT_FILE || (echo "File not found: $CONTRACT_FILE" && exit -1)
	just _cargows test -p service {{args}}

indexer-tests *args:
	just _cargows test -p indexer -- --test-threads=1 {{args}}

# Dev

dev: build-contract restart-docker gen-schema-graphql run-graphql

dev-min: start-docker-db-only run-graphql

_cargows cmd *args:
	cargo {{cmd}} --manifest-path workspace/Cargo.toml {{args}}

check: 
	just _cargows check --tests

check-watch: 
	fd .rs | entr just check

build-contract:
	cargo build-sbf --manifest-path program/nezha-staking/Cargo.toml --features dev
	cargo build-sbf --manifest-path program/nezha-vrf-mock/Cargo.toml 
	cargo build-sbf --manifest-path program/nezha-vrf/Cargo.toml 

start-docker-db-only:
	docker compose up -d postgres

start-docker:
	docker compose up -d
	docker compose logs --follow validator-setup

stop-docker:
	docker compose down

restart-docker: build-contract
	docker compose down
	docker compose up -d
	docker compose logs --follow validator-setup

restart-docker-build: build-contract
	docker compose down
	docker compose up --build -d
	docker compose logs --follow validator-setup

run-graphql:
	RUST_LOG=info,hyper=off just _cargows run -p api -j4 --bin api

gen-schema-graphql:
	just _cargows run -p api -j4 --bin gen-schema

# Git

# Push the program folder to contracts repo using `git subtree push`
push-contract:
	git subtree push -P program contracts main
