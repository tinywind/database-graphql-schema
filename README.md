# database-graphql-schema

Rust-native database-to-GraphQL SDL generation for database-backed entity types and native enums.

The migrated relational database schema is the persisted-entity source of truth:

```text
migrated relational database schema
├─ SeaORM DB-first codegen       → Rust Entity / Enum
└─ database-graphql-schema       → GraphQL base entity / enum SDL
```

This project is deliberately **not** an ORM, GraphQL server framework, or automatic CRUD API. It generates the structural base entity contract that a consumer can extend with handwritten capability SDL.

## Supported databases

The 0.2.x line has real integration fixtures and SeaORM 2.0.4 compatibility gates for:

| Database | Tested release | Metadata/features |
| --- | --- | --- |
| PostgreSQL | 18.6 | schema enums, comments, arrays/multidimensional arrays, PK/FK, numeric metadata |
| MySQL | 26.10.0 | inline ENUM, table/column comments, PK/FK, numeric metadata |
| MariaDB | 13.0.2 | MySQL-protocol adapter, inline ENUM, comments, PK/FK, numeric metadata |
| SQLite | 3.54.1 (bundled via rusqlite 0.41.1) | tables/columns, PK/FK, declared types; SQLite has no native schema enum/comment catalog |

SQLite metadata uses patched bundled SQLite 3.54.1 through rusqlite 0.41.1; PostgreSQL/MySQL-family metadata uses SQLx 0.9.0.

The production metadata layer is adapter-based. Additional DBMSes can be added without changing the generator/rule/render core. Oracle and SQL Server are **not** claimed as supported in 0.2.x because they do not yet have integration and compatibility fixtures.

## What it generates

Automatically generated:

- selected database tables as GraphQL object types;
- selected scalar columns;
- database nullability;
- native enums where the DBMS has them;
- PostgreSQL arrays, including multidimensional arrays;
- table/column/enum comments where the DBMS catalog stores them;
- deterministic canonical SDL.

Not generated:

- relations;
- computed fields;
- Query / Mutation / Subscription;
- input objects;
- authorization;
- business projections;
- pagination semantics;
- service workflows.

A downstream project can therefore keep:

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

This repository has no Gradle/JVM/Node runtime dependency and never connects to a database from `build.rs`.

## Distribution

Reusable library:

```toml
[dependencies]
database-graphql-schema = "0.2.0"
```

Cargo subcommand:

```bash
cargo install cargo-graphql-schema --version 0.2.0 --locked

cargo graphql-schema generate --config graphql-schema.toml
cargo graphql-schema verify --config graphql-schema.toml
cargo graphql-schema print --config graphql-schema.toml
```

## Configuration

The DBMS is normally inferred from `DATABASE_URL`; `driver` can be set explicitly.

```toml
[database]
driver = "auto" # auto | postgres | mysql | sqlite
schema = "public" # optional: PostgreSQL default public, SQLite default main, MySQL defaults URL database
url_env = "DATABASE_URL"

[generator]
output = "src/graphql/entities.graphqls"
descriptions = true

[tables]
# At least one include is mandatory.
include = [".*"]
exclude = ["internal_.*"]

[columns]
exclude_foreign_keys = true
keep_primary_keys = true
# Explicit include can re-expose an FK.
include = ["project\\.owner_id"]
# Exclude always wins.
exclude = ["account\\.secret_hash"]

[types.sql]
# DBMS type-name override.
jsonb = "String"

[[types.override]]
pattern = "legacy_record\\.payload"
graphql = "String"

[types]
nullable = ["legacy_record\\.optional_value"]
non_null = ["legacy_record\\.required_value"]

[enums]
include_unreferenced = false
include = []
```

All configured regex rules are case-insensitive **full matches**. A configured rule that matches nothing is an error.

### Database URLs

```bash
# PostgreSQL
DATABASE_URL='postgresql://user:password@localhost/app'

# MySQL / MariaDB
DATABASE_URL='mysql://user:password@localhost/app'

# SQLite
DATABASE_URL='sqlite:///absolute/path/app.sqlite'
```

`mariadb://` is also accepted and normalized to the MySQL protocol backend.

The preferred secret sources are the configured environment variable or `--database-url-file`. `--database-url` is supported but can expose credentials through shell/process inspection. Connection errors sanitize full DSNs, passwords, and query values.

## Generate

```bash
cargo graphql-schema generate --config graphql-schema.toml
```

Generation introspects the current migrated logical schema and writes canonical SDL to the configured output.

## Verify

```bash
cargo graphql-schema verify --config graphql-schema.toml
```

Verification regenerates SDL in memory and compares it byte-for-byte with the output file.

It fails when:

- the output file is missing;
- contents differ;
- a rule is stale;
- an SQL type is unsupported and has no explicit mapping;
- nullable and non-null overrides conflict;
- a GraphQL name/value is invalid or reserved;
- multiple database objects collapse to an incompatible GraphQL name.

For differing output the first changed line plus expected/actual text is reported.

## Fail-closed contract

Configuration expresses API decisions, not best-effort hints. For example:

```toml
[columns]
exclude = ["user\\.password_hash"]
```

If that field is renamed in the database, generation fails instead of silently exposing a different shape or leaving a dead rule.

Stale matching is enforced for:

- `tables.include`
- `tables.exclude`
- `columns.include`
- `columns.exclude`
- `[[types.override]]`
- `types.nullable`
- `types.non_null`
- `enums.include`

Unknown DBMS types never fall back to `String`. Add a mapping/override or exclude the column explicitly.

## DBMS type behavior

### PostgreSQL

Key defaults include `uuid→UUID`, small/integer→`Int`, bigint→`Long`, numeric/decimal→`BigDecimal`, temporal types, JSON/JSONB, bytea, native enums, and arrays.

PostgreSQL array dimensions are preserved from the database catalog. SeaORM 2.0.4 currently renders a PostgreSQL `integer[][]` model as `Vec<i32>`; this generator deliberately keeps the DB truth as `[[Int!]!]`. The compatibility gate makes that known difference visible.

### MySQL / MariaDB

Integer widths, Decimal, temporal types, JSON/text, binary/blob and inline `ENUM` are supported.

MySQL/MariaDB inline enum names follow SeaORM's column-name convention: a `status ENUM(...)` column produces GraphQL `Status`. Compatible same-named enums are deduplicated; incompatible ones fail closed.

MySQL's `BOOLEAN` is an alias of `TINYINT(1)`, and information_schema does not retain whether the original DDL used the word `BOOLEAN`. SeaORM 2.0.4 therefore generates `i8`; the default GraphQL mapping is `Int`. If the application owns semantic boolean intent, use a column override to `Boolean`.

MariaDB exposes its JSON alias as LONGTEXT in current catalog/codegen behavior, so the default GraphQL field is `String`, matching SeaORM's generated Rust `String`. MySQL native JSON maps to `JSON`.

### SQLite

SQLite is typed by declared type/affinity rather than a server catalog. The adapter follows the same DB-first type normalization used by the SeaORM/sea-schema ecosystem. SQLite `INTEGER` maps to GraphQL `Long` because the DB value and SeaORM model are signed 64-bit.

For explicit strong types with SeaORM-compatible mapping, declarations such as `uuid_text`, `json_text`, `date_text`, `datetime_text`, and `boolean` can be used. Arbitrary custom declarations remain fail-closed unless configured.

SQLite does not persist table/column comments or schema-level enums in its standard catalog, so those features are not fabricated.

## Numeric compatibility

The old JVM PostgreSQL generator 0.1.1 narrowed `numeric(scale=0)` according to precision to mirror jOOQ. SeaORM 2.0.x represents PostgreSQL/MySQL/SQLite decimal columns as Rust `Decimal`; this project therefore defaults numeric/decimal to GraphQL `BigDecimal`.

The PostgreSQL legacy compatibility fixture explicitly maps numeric to BigDecimal so all other legacy behavior can be checked byte-for-byte.

## SeaORM compatibility

SeaORM is a black-box compatibility oracle, not a runtime dependency.

Release verification runs actual `sea-orm-cli 2.0.4` DB-first codegen against the same synthetic fixtures for PostgreSQL, MySQL, MariaDB, and SQLite, then compares:

- table/entity naming;
- column naming;
- native enum identity/value behavior;
- nullability;
- PK/FK classification;
- numeric, temporal, JSON/text, binary, UUID and arrays where supported.

This makes ORM codegen drift visible without coupling production code to private SeaORM internals.

## Consumer build integration

Keep live DB generation explicit:

```make
generate:
	cargo graphql-schema generate --config graphql-schema.toml

verify:
	cargo graphql-schema verify --config graphql-schema.toml
```

Do not make ordinary `cargo build` depend on a live database.

## Library API

The `database-graphql-schema` crate exposes:

- `Config::from_path` / `Config::from_toml`;
- `DatabaseMetadataReader`;
- `detect_database_kind`;
- `Generator` for in-memory `SchemaModel`;
- `render_from_database`;
- `generate_from_database`;
- `verify_from_database`.

Expected config/schema errors use typed `Error`; predictable user errors are not modeled as panics.

## Development

Prerequisites: Rust 1.99.1, Docker for server DB integration, and SeaORM CLI 2.0.4 for compatibility verification.

```bash
make bootstrap
make verify-push
make verify
```

`make verify-push` is database-independent. `make verify` additionally validates all supported database families and runs security/license checks.

## License and attribution

Apache-2.0. PostgreSQL behavior compatibility was developed from the Apache-2.0 `tinywind/GRAPHQL-SCHEMA-GENERATOR` 0.1.1 implementation and tests. See [NOTICE](NOTICE) and [docs/compatibility.md](docs/compatibility.md).
