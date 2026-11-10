# Library API

The `database-graphql-schema` crate is the reusable layer behind the Cargo subcommand. Its public API is intentionally small.

## Configuration

`Config::from_path(path)` parses TOML, validates values, and resolves relative output paths against the config file.

`Config::from_toml(source)` is suitable for embedded tooling/tests.

`DatabaseConfig.driver` can be `auto`, `postgres`, `mysql`, or `sqlite`. Auto mode recognizes PostgreSQL, MySQL/MariaDB and SQLite URL schemes.

## Live database helpers

All live helpers are async and accept the database URL explicitly:

```rust,no_run
# use database_graphql_schema::{Config, render_from_database};
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let config = Config::from_path("graphql-schema.toml")?;
let url = std::env::var("DATABASE_URL")?;
let sdl = render_from_database(&config, &url).await?;
println!("{sdl}");
# Ok(())
# }
```

- `render_from_database`: canonical SDL without writing.
- `generate_from_database`: write output and return path/counts.
- `verify_from_database`: compare canonical SDL with output.
- `DatabaseMetadataReader::read`: resolve the configured backend and produce `SchemaModel`.
- `detect_database_kind`: URL-based backend detection.

The library never reads `DATABASE_URL` itself. Environment/file/CLI secret resolution belongs to the caller.

## In-memory generation

A caller can construct a backend-independent `SchemaModel` and call `Generator::render`, `Generator::generate`, or `Generator::verify`. This separation keeps rule/type/naming/render logic testable without a live DBMS.

`SchemaModel.database_kind` controls built-in SQL type mapping. Metadata adapters normalize backend-specific catalogs into that common model.

## Errors

Expected config/schema/metadata failures return `database_graphql_schema::Error`. Production generation does not panic for expected user errors.

Connection errors sanitize the original DSN/password/query values before exposure.

## SemVer

The 0.x series may evolve Rust API with minor releases. Observable generator/CLI contract changes are documented in CHANGELOG.md and docs/compatibility.md and require backend compatibility tests.
