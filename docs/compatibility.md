# Compatibility contract

## Legacy GRAPHQL-SCHEMA-GENERATOR 0.1.1

The Rust implementation preserves these observable semantics from the Apache-2.0 JVM generator:

- explicit table include is mandatory;
- case-insensitive full-match regex rules;
- table/column exclude precedence;
- FK filtering and PK+FK retention;
- explicit FK column re-inclusion;
- ordered column type overrides;
- nullable/non-null overrides and conflict failure;
- referenced-enum inclusion;
- deterministic enum/type sorting and database column order;
- comments to GraphQL descriptions;
- stale rule failure;
- unknown type failure;
- naming/field collision failure;
- missing/different verify behavior with first-difference reporting.

The synthetic fixture in `fixtures/postgres/fixture.sql` contains no private application schema. The legacy generator is executed out-of-repository during release verification and its output is compared with `fixtures/java-0.1.1/expected.graphqls`.

### Intentional numeric difference

0.1.1 used jOOQ's zero-scale numeric width behavior: precision <= 9 → Int, <= 18 → Long, otherwise BigInteger. SeaORM 2.0.4 generates PostgreSQL numeric as Rust Decimal for every scale. The Rust generator therefore defaults every numeric/decimal to GraphQL `BigDecimal`.

For the byte-for-byte legacy compatibility fixture, both generators explicitly configure `numeric -> BigDecimal`. Default numeric behavior is separately unit-tested and SeaORM-tested.

## SeaORM 2.0.4

Release verification invokes the actual `sea-orm-cli 2.0.4` DB-first entity generator against the synthetic PostgreSQL fixture. The gate compares:

- database table → SeaORM entity/model name → GraphQL object type;
- database column → SeaORM model field → GraphQL lower-camel field;
- PostgreSQL enum name and database string values;
- Rust `Option<T>` vs GraphQL nullable field;
- PK/FK classification used by GraphQL filtering;
- UUID, Decimal, temporal, JSON, INET, and array representation.

The production library does not depend on `sea-orm-codegen` internals. This keeps runtime dependencies small and makes compatibility a black-box contract rather than a private-API dependency.

## Arrays

PostgreSQL schema metadata records column nullability and array dimensions but does not declare a non-null constraint for individual array elements. For compatibility with the predecessor generator and SeaORM's `Vec<T>` representation, generated lists use non-null element syntax such as `[String!]` and `[[Int!]!]`. Applications that intentionally store null array elements should override that field's GraphQL type.

## Unsupported/custom types

Domains, composites, ranges, vectors, extension types, and future PostgreSQL types are fail-closed unless a built-in mapping exists. Add `[types.sql]`, a `[[types.override]]`, or a column exclusion explicitly.

### Legacy citext fixture detail

The shared synthetic fixture installs PostgreSQL `citext` in `public` because SeaORM 2.0.4 cannot discover a `citext` column when that extension type lives in a separate schema. jOOQ, used by the legacy 0.1.1 oracle, also discovers the extension's overloaded `regexp_*` set-returning functions as table-like objects. The external legacy-oracle harness therefore excludes `(?i:regexp_.*)` at the jOOQ database-metadata layer only. The five synthetic application tables and the generator's table/column rules are unchanged, and the resulting legacy SDL is compared byte-for-byte with the Rust output.
