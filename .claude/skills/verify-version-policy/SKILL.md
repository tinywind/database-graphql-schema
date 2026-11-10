# Verify version policy

This skill reports version status; it does not automatically update dependencies.

For Rust, PostgreSQL, SeaORM, runtime dependencies, GitHub Actions, cargo-deny, and cargo-audit:
1. record the current repository pin;
2. check official upstream/current registry latest stable (pre-releases are not stable);
3. assess compatibility with this repository;
4. classify each difference as:
   - `AGENT_FIX`: clearly safe and required,
   - `NEEDS_DECISION`: behavior/compatibility choice,
   - `EVIDENCE_REQUIRED`: current evidence is insufficient.

If an older major is intentionally required, report blocker, latest version, selected version, upstream evidence, and removal condition. Never update a dependency solely because this skill ran.
