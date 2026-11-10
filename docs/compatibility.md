# Compatibility contract

## Cross-database invariant

Every supported metadata adapter produces the same logical `SchemaModel`. Rule application, naming validation, deterministic ordering, rendering, and verify behavior are DBMS-independent.

The following remain fail-closed across all backends:

- explicit table include is mandatory;
- case-insensitive full-match regex rules;
- exclude precedence;
- FK filtering and PK+FK retention;
- explicit FK re-inclusion;
- type and nullability overrides;
- stale rule detection;
- unknown type failure;
- invalid/reserved/colliding GraphQL names;
- byte-identical deterministic output;
- missing/different verify semantics.

## Tested DBMS matrix

| Backend | Tested version | Native enum model | DB comments |
| --- | --- | --- | --- |
| PostgreSQL | 18.6 | schema type | table/column/enum |
| MySQL | 26.10.0 | inline ENUM | table/column |
| MariaDB | 13.0.2 | inline ENUM | table/column |
| SQLite | 3.54.1 (bundled runtime) | none | standard catalog has none |

Oracle and SQL Server have no adapter/fixture in 0.2.x and are not advertised as supported.

## SeaORM 2.0.4

Release verification invokes actual `sea-orm-cli 2.0.4` against every synthetic fixture rather than linking production code to codegen internals.

The compatibility gate checks table/entity naming, column fields, nullability, PK/FK behavior and representative type mappings.

### PostgreSQL

- native enum names/values align;
- UUID, Decimal, temporal, JSON and arrays are checked;
- PostgreSQL multidimensional array dimensions are preserved by this generator even though SeaORM 2.0.4 currently emits a one-dimensional Rust `Vec<T>` for a multidimensional DB array.

### MySQL

- BIGINT aligns with Rust i64 / GraphQL Long;
- Decimal, JSON, temporal and binary types are checked;
- inline `status ENUM(...)` aligns with SeaORM active enum `Status`;
- `BOOLEAN` is metadata-equivalent to TINYINT(1), so SeaORM emits i8 and the generator defaults to GraphQL Int.

### MariaDB

The same MySQL-protocol metadata adapter is used, but differences in actual MariaDB catalog semantics are preserved. In MariaDB 13.0.2, JSON is exposed/code-generated as LONGTEXT/String rather than MySQL's native Json model; GraphQL follows that DB/codegen truth.

### SQLite

SQLite INTEGER aligns with SeaORM i64 / GraphQL Long. Boolean, Decimal, JSON_TEXT, temporal marker declarations and BLOB are checked against actual SeaORM output.

## Legacy GRAPHQL-SCHEMA-GENERATOR 0.1.1

The PostgreSQL adapter preserves the observable semantics of the Apache-2.0 JVM generator where they remain appropriate:

- table/column rules;
- FK/PK filtering;
- type/nullability override behavior;
- enum selection;
- deterministic order;
- comments/descriptions;
- stale-rule and unknown-type failures;
- collision handling;
- verify mismatch reporting.

The synthetic PostgreSQL fixture contains no private application schema. The checked golden output is compared byte-for-byte in integration tests.

### Intentional numeric difference

The JVM 0.1.1 generator narrowed zero-scale numeric by precision because jOOQ generated corresponding integral POJO types. SeaORM 2.0.x uses Decimal for the tested database decimal/numeric types. The new generator therefore defaults decimal/numeric to GraphQL `BigDecimal`.

The legacy byte-for-byte fixture explicitly configures numeric→BigDecimal to isolate and test every other compatibility behavior.

## Arrays

PostgreSQL is the only 0.2.x backend with native array support. Generated lists use non-null element syntax such as `[String!]` and `[[Int!]!]`. PostgreSQL does not provide per-element nullability metadata, so applications intentionally storing null array elements should use an explicit column type override.

## Unsupported/custom types

A type that is not in a backend's built-in mapping remains unmapped. The consumer must add `[types.sql]`, a `[[types.override]]`, or exclude the column.

There is no universal String fallback.
