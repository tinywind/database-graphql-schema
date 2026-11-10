# Verify version policy

This skill reports version status; it does not auto-update dependencies.

Check current pins against official upstream/latest stable for Rust, PostgreSQL, MySQL, MariaDB, SQLite, SeaORM, runtime Rust dependencies, GitHub Actions, cargo-deny and cargo-audit.

Classify differences:
- `AGENT_FIX`: clearly safe and required;
- `NEEDS_DECISION`: behavior/compatibility meaning changes;
- `EVIDENCE_REQUIRED`: current evidence is insufficient.

If an older version is intentionally required, record blocker, upstream latest, selected version, evidence and removal condition. Pre-releases are not stable.
