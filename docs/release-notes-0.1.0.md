# postgres-graphql-schema 0.1.0

Historical first release of the PostgreSQL-focused generator before the project was generalized and renamed in 0.2.0.

## Highlights

- Reusable `postgres-graphql-schema` library plus `cargo-graphql-schema` Cargo subcommand.
- `generate`, `verify`, and `print`.
- PostgreSQL catalog introspection for tables, scalar columns, PK/FK membership, enums, comments, arrays, and numeric metadata.
- Fail-closed full-match table/column/type/enum rules with stale-rule detection.
- Deterministic SDL and first-line verify diagnostics.
- Credential-safe connection error rendering.
- PostgreSQL 18.6 integration tests.
- SeaORM 2.0.4 compatibility gate.
- Byte-for-byte compatibility fixture against GRAPHQL-SCHEMA-GENERATOR 0.1.1.

0.2.0 renamed the core crate/repository to `database-graphql-schema` and added MySQL, MariaDB and SQLite backends.
