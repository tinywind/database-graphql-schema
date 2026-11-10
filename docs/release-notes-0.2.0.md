# 0.2.0

0.2.0 turns the original PostgreSQL-focused generator into a multi-DBMS relational schema generator.

## Highlights

- Project/repository/core crate renamed to `database-graphql-schema`.
- PostgreSQL 18.6, MySQL 26.10.0, MariaDB 13.0.2 and SQLite support.
- Shared fail-closed generation core with DBMS-specific metadata adapters.
- SQLite uses patched bundled SQLite 3.54.1 via rusqlite 0.41.1.
- Native PostgreSQL enums and MySQL/MariaDB inline ENUM.
- Cross-DB SeaORM 2.0.4 compatibility verification.
- Explicit DBMS-specific handling for MySQL BOOLEAN/TINYINT and MariaDB JSON/LONGTEXT.
- PostgreSQL legacy 0.1.1 compatibility retained.

## CLI

```bash
cargo graphql-schema generate --config graphql-schema.toml
cargo graphql-schema verify --config graphql-schema.toml
cargo graphql-schema print --config graphql-schema.toml
```

## Known limitations

- Oracle and SQL Server do not yet have adapters or compatibility fixtures.
- PostgreSQL arrays have no per-element nullability metadata.
- SeaORM 2.0.4 flattens PostgreSQL multidimensional arrays in Rust codegen; GraphQL intentionally preserves DB dimensions.
- SQLite standard schema metadata has no table/column comments or native enums.
- MySQL/MariaDB BOOLEAN intent is not distinguishable from TINYINT(1) through information_schema; use a GraphQL override for semantically boolean fields.
