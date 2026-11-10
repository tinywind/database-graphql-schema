SHELL := bash

.DEFAULT_GOAL := help

.PHONY: help bootstrap hooks-install format format-check test test-databases test-postgres test-mysql-family test-sqlite security package publish-dry-run verify-push verify clean

help:
	@printf '%s\n' \
	  'make bootstrap       Fetch dependencies and install repository hooks' \
	  'make verify-push     Run database-independent formatting/lint/test/doc/package gates' \
	  'make verify          Run all DBMS, SeaORM, security and package gates' \
	  'make test-databases  Run PostgreSQL, MySQL, MariaDB and SQLite compatibility' \
	  'make security        Run cargo-deny and cargo-audit'

bootstrap:
	@command -v cargo >/dev/null
	@cargo fetch --locked
	@$(MAKE) hooks-install

hooks-install:
	@git config core.hooksPath .githooks
	@chmod +x .githooks/pre-push scripts/*.sh 2>/dev/null || true

format:
	@cargo fmt --all

format-check:
	@cargo fmt --all --check

test:
	@cargo test --all-features
	@cargo test --doc

test-postgres:
	@scripts/verify-postgres.sh

test-mysql-family:
	@scripts/verify-mysql-family.sh

test-sqlite:
	@scripts/verify-sqlite.sh

test-databases:
	@scripts/verify-databases.sh

security:
	@command -v cargo-deny >/dev/null || { echo 'cargo-deny is required (pinned: 0.20.2)' >&2; exit 1; }
	@command -v cargo-audit >/dev/null || { echo 'cargo-audit is required (pinned: 0.23.0)' >&2; exit 1; }
	@cargo deny check
	@cargo audit

package:
	@scripts/package.sh

publish-dry-run:
	@scripts/publish-dry-run.sh

verify-push: format-check
	@cargo check --all-targets
	@cargo clippy --all-targets --all-features -- -D warnings
	@$(MAKE) test
	@$(MAKE) package
	@$(MAKE) publish-dry-run

verify: verify-push
	@$(MAKE) test-databases
	@$(MAKE) security

clean:
	@cargo clean
	@rm -rf .tmp
