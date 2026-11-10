# postgres-graphql-schema

A Rust-native PostgreSQL-to-GraphQL SDL generator for database-backed entity types and enums.

The migrated PostgreSQL logical schema is the persisted-entity source of truth:

```text
PostgreSQL logical schema
├─ SeaORM DB-first codegen → Rust Entity / Enum
└─ postgres-graphql-schema → GraphQL entity / enum SDL
```

The project is deliberately **not** an ORM, GraphQL server framework, or CRUD API generator. It generates the structural base that a consumer can extend with its own capability contract.

## What it generates

- PostgreSQL tables as GraphQL object types.
- Public scalar columns selected by fail-closed rules.
- PostgreSQL enums referenced by generated fields, plus explicitly selected enums.
- Column nullability.
- PostgreSQL arrays, including multidimensional arrays.
- Table, column, and enum comments as GraphQL descriptions.
- Deterministically sorted declarations with database column order preserved.

It does **not** generate relations, computed fields, Query, Mutation, Subscription, input objects, authorization, pagination semantics, or business projections. Those belong to the consumer.

A typical downstream shape is:

```text
entities.graphqls
+
handwritten capability SDL
        ↓
full GraphQL schema
        ↓
GraphQL operation generation
        ├─ Web TypeScript codegen
        └─ Android Apollo Kotlin codegen
```

This repository has no Node, Apollo, Gradle, JVM, Java, or Kotlin runtime dependency.

## Install

The Cargo subcommand is published as `cargo-graphql-schema`:

```bash
cargo install cargo-graphql-schema --version 0.1.0 --locked
```

Cargo discovers binaries named `cargo-<name>`, so the installed binary can be invoked naturally:

```bash
cargo graphql-schema generate --config graphql-schema.toml
cargo graphql-schema verify --config graphql-schema.toml
cargo graphql-schema print --config graphql-schema.toml
```

The reusable library crate is `postgres-graphql-schema`.

## Configuration

`graphql-schema.toml`:

```toml
[database]
schema = "public"
url_env = "DATABASE_URL"

[generator]
output = "src/graphql/entities.graphqls"
descriptions = true

[tables]
# At least one include rule is required.
include = [".*"]
exclude = ["internal_.*"]

[columns]
exclude_foreign_keys = true
keep_primary_keys = true
# An explicit include may expose an FK column.
include = ["project\\.owner_id"]
# Exclude wins over every include/filtering rule.
exclude = ["account\\.secret_hash"]

[types.sql]
# Optional SQL type mappings replace built-ins.
jsonb = "String"

[[types.override]]
pattern = "legacy_record\\.payload"
graphql = "String"

[types]
nullable = ["legacy_record\\.optional_value"]
non_null = ["legacy_record\\.required_value"]

[enums]
include_unreferenced = false
include = ["explicitly_public_enum"]
```

All regex rules are case-insensitive **full matches**, not substring searches. Table excludes win over includes. Column excludes win over explicit includes. FK columns are hidden by default; FK columns that are also PK columns remain unless `keep_primary_keys = false`.

The database URL is not required in the config. Resolution order is:

1. `--database-url` (supported, but not recommended for secrets because shell history/process inspection may expose it),
2. `--database-url-file`,
3. `--database-url-env NAME`,
4. the environment variable named by `database.url_env` (default `DATABASE_URL`).

Connection errors are sanitized so the full DSN, password, and query values are not emitted by this tool.

## Generate

```bash
DATABASE_URL='postgresql://...' cargo graphql-schema generate --config graphql-schema.toml
```

Generation reads live PostgreSQL catalog metadata and replaces the configured output with canonical SDL. It never runs during an ordinary `cargo build`.

## Verify

```bash
DATABASE_URL='postgresql://...' cargo graphql-schema verify --config graphql-schema.toml
```

Verification regenerates SDL in memory and compares it byte-for-byte with the committed/generated output.

It fails when:

- the output file is missing;
- the content differs;
- a configured rule is stale;
- a selected SQL type has no mapping;
- a GraphQL name/literal is invalid or reserved;
- two database objects map to the same GraphQL type/field name.

For a content mismatch, the first differing line and expected/actual text are reported.

## Fail-closed rules

Configuration is part of the public-schema contract. A rule that no longer matches is therefore an error rather than a no-op.

For example, if:

```toml
[columns]
exclude = ["user\\.password_hash"]
```

stops matching because the database column was renamed, generation fails. The same stale-rule check applies to:

- `tables.include`
- `tables.exclude`
- `columns.include`
- `columns.exclude`
- `[[types.override]]`
- `types.nullable`
- `types.non_null`
- `enums.include`

Unsupported SQL types also fail closed. Add an explicit mapping/override or exclude the column.

## Default PostgreSQL mappings

| PostgreSQL | GraphQL |
| --- | --- |
| `uuid` | `UUID` |
| varchar/char/text/name/citext/inet/cidr/macaddr/macaddr8 | `String` |
| smallint/integer | `Int` |
| bigint | `Long` |
| boolean | `Boolean` |
| real/double precision | `Float` |
| numeric/decimal | `BigDecimal` |
| date | `Date` |
| timestamp | `LocalDateTime` |
| timestamptz | `DateTime` |
| time | `LocalTime` |
| timetz | `Time` |
| json/jsonb | `JSON` |
| PostgreSQL enum | generated GraphQL enum |
| array | nested GraphQL list |

The generator emits scalar *uses* but does not declare custom GraphQL scalars. The GraphQL runtime/capability schema owns scalar definitions and serialization.

### Numeric compatibility decision

The legacy JVM generator 0.1.1 narrowed `numeric(scale=0)` to `Int`, `Long`, or `BigInteger` using precision because that mirrored jOOQ POJO codegen. SeaORM 2.0.x generates PostgreSQL numeric as Rust `Decimal` regardless of scale. This Rust generator therefore preserves decimal semantics as `BigDecimal` for every numeric/decimal column. A consumer can override it explicitly when its API contract intentionally narrows a value.

The checked compatibility fixture maps numeric explicitly to `BigDecimal` so every other legacy behavior can be compared byte-for-byte.

## SeaORM alignment

SeaORM 2.0.x codegen derives table/entity and enum names with upper-camel casing and column fields with snake casing. This project derives GraphQL object/enum names from the same database names and GraphQL field names with lower-camel casing.

Release verification runs SeaORM's actual CLI against the same PostgreSQL fixture and checks table/column/enum/type/nullability alignment. This is a golden compatibility gate: a future SeaORM naming/type drift becomes a test failure instead of silently changing an API.

## Consumer integration

Keep generation explicit in the consumer build graph:

```make
generate:
	cargo graphql-schema generate --config graphql-schema.toml

verify:
	cargo graphql-schema verify --config graphql-schema.toml
```

Do not put a live database connection in `build.rs`.

## Library API

The `postgres-graphql-schema` crate exposes:

- `Config::from_path` / `Config::from_toml`
- `PostgresMetadataReader`
- `Generator` for in-memory schema models
- `render_from_database`
- `generate_from_database`
- `verify_from_database`
- naming helpers used by compatibility tests

All expected configuration/schema failures return typed `Error` values; user errors are not modeled as panics.

## Development

Prerequisites: Rust 1.99.1 and Docker.

```bash
make bootstrap
make verify-push
make verify
```

`make verify-push` is the DB-independent pre-push gate. `make verify` additionally starts an isolated PostgreSQL 18.6 fixture container and runs PostgreSQL, CLI, determinism, legacy-generator, and SeaORM compatibility verification.

Security/license checks use `cargo-deny` and `cargo-audit`.

## License and attribution

Apache-2.0. Behavior compatibility was developed from the Apache-2.0 `tinywind/GRAPHQL-SCHEMA-GENERATOR` 0.1.1 implementation and tests. See [NOTICE](NOTICE) and [docs/compatibility.md](docs/compatibility.md).
