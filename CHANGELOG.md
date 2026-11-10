# Changelog

All notable changes are documented here. This project follows Semantic Versioning.

## 0.2.0 - 2026-11-11

- Renamed the project and core crate from PostgreSQL-specific `postgres-graphql-schema` to `database-graphql-schema`.
- Added an adapter-based metadata architecture with automatic DBMS selection.
- Added tested PostgreSQL 18.6, MySQL 26.10.0, MariaDB 13.0.2, and SQLite support.
- Added MySQL/MariaDB inline ENUM generation and catalog comments.
- Added SQLite declared-type, PK/FK, numeric and deterministic generation support using patched bundled SQLite 3.54.1 via rusqlite 0.41.1.
- Added cross-DB SeaORM 2.0.4 compatibility gates and compiled CLI functional tests.
- Preserved the PostgreSQL legacy 0.1.1 byte-for-byte compatibility fixture.
- Added cross-DB scalar mappings for binary/blob values and documented DBMS-specific type semantics.

## 0.1.0 - 2026-11-11

- Initial PostgreSQL-focused reusable Rust library and Cargo subcommand.
- PostgreSQL catalog introspection for tables, columns, PK/FK, enums, comments, arrays, and numeric metadata.
- Deterministic GraphQL base entity/enum SDL generation.
- Fail-closed table/column/type/enum rule matching.
- Generate, verify, and print commands.
- PostgreSQL 18.6 integration and CLI functional tests.
- SeaORM 2.0.4 and legacy generator 0.1.1 compatibility gates.
- Credential-safe connection errors.
