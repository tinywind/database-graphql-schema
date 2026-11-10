# Verify generator contract

Review the public contract without weakening it.

Run `cargo test --all-features` and inspect coverage for table rules, FK/PK filtering, column precedence, type overrides, nullability conflicts, enum selection, stale rules, collisions, unknown types, descriptions, deterministic rendering, verify missing/different, and DSN redaction.

For metadata behavior, run `scripts/verify-databases.sh` when Docker is available. PostgreSQL, MySQL, MariaDB and SQLite are supported only because they have real fixtures.

A missing fail-closed regression is a finding. Do not make a stale rule optional to pass a test.
