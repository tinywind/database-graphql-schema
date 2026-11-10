# postgres-graphql-schema 0.1.0

Initial release of a Rust-native PostgreSQL-to-GraphQL base entity SDL generator.

## Highlights

- Reusable `postgres-graphql-schema` library plus `cargo-graphql-schema` Cargo subcommand.
- `generate`, `verify`, and `print` commands.
- PostgreSQL catalog introspection for tables, scalar columns, PK/FK membership, enums, comments, arrays, and numeric metadata.
- Fail-closed full-match table/column/type/enum rules with stale-rule detection.
- Deterministic SDL and first-line verify diagnostics.
- Credential-safe connection error rendering.
- Actual PostgreSQL 18.6 integration tests.
- Actual SeaORM 2.0.4 codegen compatibility gate.
- Byte-for-byte compatibility fixture against GRAPHQL-SCHEMA-GENERATOR 0.1.1 for shared semantics.

## CLI

```bash
cargo install cargo-graphql-schema --version 0.1.0 --locked
cargo graphql-schema generate --config graphql-schema.toml
cargo graphql-schema verify --config graphql-schema.toml
cargo graphql-schema print --config graphql-schema.toml
```

## PostgreSQL / SeaORM compatibility

The release verification fixture runs on PostgreSQL 18.6 and SeaORM CLI 2.0.4. Metadata access uses PostgreSQL catalogs directly through SQLx 0.9.0 rather than SeaORM private codegen APIs.

SeaORM 2.0.4 currently renders a PostgreSQL multidimensional array such as `integer[][]` as `Vec<i32>`. This generator preserves PostgreSQL's declared dimensions and emits `[[Int!]!]`; the compatibility gate treats that known difference explicitly.

PostgreSQL does not encode element-level array nullability constraints. Lists therefore use non-null element syntax for compatibility with the predecessor generator and Rust `Vec<T>`; consumers that intentionally store null array elements should override the field type.

## Compatibility expectations

The generator produces only DB-backed base entity fields and PostgreSQL enums. It does not infer relations, computed fields, operations, inputs, authorization, pagination, or business projections.

Custom/domain/composite/range/extension types are fail-closed unless built in, explicitly mapped/overridden, or excluded. The default numeric mapping is `BigDecimal` for all PostgreSQL numeric/decimal columns to align with SeaORM's Rust `Decimal` semantics. This intentionally differs from the predecessor JVM generator's default zero-scale precision narrowing.

The 0.x public Rust API may evolve in minor releases. Observable generator/CLI contract changes will be documented and covered by compatibility tests.
