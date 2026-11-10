# Verify generator contract

Verify the generator's public behavior without changing source.

Run:
1. `cargo test --all-features`
2. Inspect tests for table include/exclude, FK/PK filtering, column precedence, type overrides, nullable/non-null conflicts, enum selection, stale patterns, collisions, unknown types, arrays, descriptions, deterministic rendering, verify missing/different, and DSN redaction.
3. Run the PostgreSQL-backed gate when Docker is available: `scripts/verify-postgres.sh`.

Report PASS only when the relevant checks actually ran. A missing fail-closed regression is a finding, not a reason to weaken validation.
