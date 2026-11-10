# database-graphql-schema agent instructions

Read README.md and docs/compatibility.md before changing behavior. The migrated relational database schema is the persisted-entity SSOT. This repository generates only GraphQL base entity scalar fields and native enums; relations, operations, inputs, authorization, pagination and business projections belong to consumers.

Supported 0.2.x metadata backends are PostgreSQL, MySQL/MariaDB and SQLite. Keep the generator/rule/render core DBMS-independent and isolate catalog behavior in metadata adapters. Do not claim support for a DBMS without a real fixture and compatibility test.

Preserve fail-closed invariants: explicit table includes, stale-rule failures, unknown-type failures, name/literal collision failures, deterministic output, exact verify behavior and credential-safe errors.

The production implementation is Rust-only. Do not add Gradle/JVM/Node runtime dependencies or live DB access from build.rs. SeaORM and the legacy JVM generator are compatibility oracles only.

Use Rust 1.99.1 and pinned dependency majors. Before push run `make verify-push`; before release or metadata/naming/type changes run `make verify`. Keep public fixtures synthetic. Never copy private application schemas, credentials, internal URLs or customer data into this repository.

Preserve unrelated work and never weaken tests to hide DBMS/ORM drift. Intentional differences require tests plus docs/compatibility.md updates.
