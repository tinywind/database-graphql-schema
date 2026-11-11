# database-graphql-schema

Generate deterministic GraphQL SDL for database-backed entity types and native enums directly from a migrated relational database schema.

In a DB-first Rust project, database migrations, ORM entities, and GraphQL SDL can drift apart. `database-graphql-schema` keeps the migrated database as the persisted-model source of truth and lets CI catch that drift before it reaches consumers.

```text
migrated relational database schema
├─ SeaORM DB-first codegen       → Rust Entity / Enum
└─ database-graphql-schema       → GraphQL base entity / enum SDL
```

It is deliberately **not** an ORM, GraphQL server framework, or automatic CRUD API. Your application still owns relations, operations, authorization, pagination, and business-specific GraphQL design.

## Quick start

Install the Cargo subcommand:

```bash
cargo install cargo-graphql-schema --version 0.2.0 --locked
```

Create `graphql-schema.toml`:

```toml
[generator]
output = "src/graphql/entities.graphqls"

[tables]
include = [".*"]
```

Generate from a migrated database:

```bash
DATABASE_URL='postgresql://user:password@localhost/app' \
  cargo graphql-schema generate --config graphql-schema.toml
```

For example:

```sql
CREATE TABLE account (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
```

becomes:

```graphql
type Account {
    id: UUID!
    email: String!
    createdAt: DateTime!
}
```

Check that the committed SDL still matches the database:

```bash
DATABASE_URL='postgresql://user:password@localhost/app' \
  cargo graphql-schema verify --config graphql-schema.toml
```

`verify` regenerates SDL in memory and exits non-zero when the output is missing or stale, the database shape changed, or a configured rule/mapping is no longer valid. This makes it suitable for CI schema-drift checks.

GraphQL scalar declarations such as `UUID`, `Long`, `BigDecimal`, `DateTime`, or `JSON` remain consumer-owned.

## Supported databases

The 0.2.x line has real integration fixtures and SeaORM 2.0.4 compatibility gates for:

| Database | Tested release | Highlights |
| --- | --- | --- |
| PostgreSQL | 18.6 | schema enums, comments, arrays, PK/FK, numeric metadata |
| MySQL | 26.10.0 | inline ENUM, comments, PK/FK, numeric metadata |
| MariaDB | 13.0.2 | MySQL-protocol adapter, inline ENUM, comments, PK/FK |
| SQLite | 3.54.1 | tables/columns, PK/FK, declared types |

The metadata layer is adapter-based. Oracle and SQL Server are not advertised as supported in 0.2.x because they do not yet have real integration/compatibility fixtures.

See [docs/compatibility.md](docs/compatibility.md) for DBMS-specific behavior and known ORM differences.

## Scope

Generated from the database:

- selected tables and scalar columns;
- nullability;
- native enums where available;
- PostgreSQL arrays;
- catalog comments where available;
- deterministic canonical SDL.

Intentionally not generated:

- relations or computed fields;
- Query / Mutation / Subscription;
- input objects;
- authorization or service workflows;
- business projections or pagination semantics.

A consumer typically combines generated entities with handwritten capability SDL:

```text
entities.graphqls
+
handwritten capability SDL
        ↓
full GraphQL schema
        ↓
operation/client code generation
```

## Configuration essentials

The DBMS is normally inferred from `DATABASE_URL`. At least one `tables.include` rule is required.

A more selective configuration can add filters and explicit mappings:

```toml
[database]
driver = "auto" # auto | postgres | mysql | sqlite
url_env = "DATABASE_URL"

[generator]
output = "src/graphql/entities.graphqls"

[tables]
include = [".*"]
exclude = ["internal_.*"]

[columns]
exclude_foreign_keys = true
keep_primary_keys = true
include = ["project\\.owner_id"]
exclude = ["account\\.secret_hash"]

[types.sql]
jsonb = "String"

[[types.override]]
pattern = "legacy_record\\.payload"
graphql = "String"
```

Configuration is intentionally fail-closed:

- regex rules are case-insensitive full matches;
- excludes win over includes;
- configured rules that match nothing are errors;
- unknown DBMS types do not silently become `String`;
- invalid/reserved/colliding GraphQL names are errors;
- conflicting nullability overrides are errors.

This means a rename cannot silently turn a security-sensitive exclusion into a dead rule.

Supported URL schemes include `postgresql://`, `postgres://`, `mysql://`, `mariadb://`, and `sqlite://`. Prefer the configured environment variable or `--database-url-file` for credentials.

## Commands

```bash
# Write the configured output file.
cargo graphql-schema generate --config graphql-schema.toml

# Compare the configured output with the live migrated schema.
cargo graphql-schema verify --config graphql-schema.toml

# Print SDL without writing the configured output.
cargo graphql-schema print --config graphql-schema.toml
```

## Consumer build integration

Keep live DB generation explicit instead of making ordinary `cargo build` depend on a database:

```make
generate:
	cargo graphql-schema generate --config graphql-schema.toml

verify:
	cargo graphql-schema verify --config graphql-schema.toml
```

## Library use

The same generator is available as a Rust library:

```toml
[dependencies]
database-graphql-schema = "0.2.0"
```

The public API exposes configuration loading, database metadata reading, the common `SchemaModel`, generation, rendering, file generation, and verification.

See [docs/library-api.md](docs/library-api.md) for embedding examples.

## Compatibility model

SeaORM is a compatibility oracle, not a runtime dependency. Release verification runs actual SeaORM 2.0.4 DB-first codegen against the same synthetic fixtures.

Details such as PostgreSQL multidimensional arrays, MySQL `BOOLEAN`/`TINYINT(1)`, MariaDB JSON behavior, SQLite type normalization, legacy JVM compatibility, and numeric mapping live in [docs/compatibility.md](docs/compatibility.md).

## Development

Prerequisites: Rust 1.99.1, Docker for server-DB integration, and SeaORM CLI 2.0.4 for compatibility verification.

```bash
make bootstrap
make verify-push
make verify
```

`make verify-push` runs database-independent formatting, lint, tests, docs, package, and publish-dry-run gates. `make verify` additionally runs all supported DBMS compatibility and security/license gates.

## Documentation

- [Compatibility contract](docs/compatibility.md)
- [Library API](docs/library-api.md)
- [0.2.0 release notes](docs/release-notes-0.2.0.md)

## License and attribution

Apache-2.0. PostgreSQL behavior compatibility was developed from the Apache-2.0 `tinywind/GRAPHQL-SCHEMA-GENERATOR` 0.1.1 implementation and tests. See [NOTICE](NOTICE) and [docs/compatibility.md](docs/compatibility.md).
