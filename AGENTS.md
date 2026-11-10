# postgres-graphql-schema agent instructions

Read README.md and docs/compatibility.md before changing generator behavior. PostgreSQL logical schema is the persisted-entity SSOT; this repository generates only GraphQL base entity scalar fields and PostgreSQL enums.

Preserve these fail-closed invariants: explicit table includes, stale-rule failures, unknown-type failures, name/literal collision failures, byte-deterministic rendering, exact verify behavior, and credential-safe errors. Do not infer relations, operations, inputs, authorization, pagination, or business projections.

The production implementation is Rust-only. Do not add Gradle/JVM/Node runtime dependencies or connect to a database from build.rs. SeaORM and the legacy JVM generator are compatibility oracles, not runtime dependencies.

Use Rust 1.99.1 and the pinned dependency majors. Before push run `make verify-push`; before release or metadata/naming/type changes run `make verify`. Keep generated compatibility fixtures synthetic. Never copy private application schemas, credentials, internal URLs, or customer data into this repository.

Preserve unrelated work. Do not weaken tests to hide a compatibility drift; update docs/compatibility.md only when the semantic difference is intentional and tested.
