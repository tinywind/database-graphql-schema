# Verify release readiness

A release is ready only after:
- `make verify` passes;
- `cargo package -p postgres-graphql-schema` passes;
- `cargo publish --dry-run -p postgres-graphql-schema` passes;
- the CLI package is packaged and dry-run published once its exact core dependency version is resolvable;
- a clean checkout repeats the full gate;
- README, CHANGELOG, license/NOTICE, crate metadata, tag, and release notes agree on the version;
- no secret/private schema/internal URL is tracked;
- the published crates are consumed from crates.io by an external temporary project and generate/verify against a fresh fixture.

Do not substitute a workspace path-dependency smoke test for the published-consumer check.
