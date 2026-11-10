# Library API

The `postgres-graphql-schema` crate is the reusable layer behind the Cargo subcommand. Its public API is intentionally small.

## Configuration

`Config::from_path(path)` parses TOML, validates required values, and resolves a relative `generator.output` relative to the config file.

`Config::from_toml(source)` is useful for embedded tooling and tests. It leaves relative output paths relative to the caller's working directory.

## Live PostgreSQL helpers

All live-database helpers are async and accept a database URL supplied by the caller:

```rust,no_run
# use postgres_graphql_schema::{Config, render_from_database};
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let config = Config::from_path("graphql-schema.toml")?;
let url = std::env::var("DATABASE_URL")?;
let sdl = render_from_database(&config, &url).await?;
println!("{sdl}");
# Ok(())
# }
```

- `render_from_database` reads metadata and returns canonical SDL without writing.
- `generate_from_database` writes the configured output and returns counts/path.
- `verify_from_database` compares canonical SDL with the configured output and returns `Verification`.

The library never reads `DATABASE_URL` itself. Environment/file/CLI credential resolution belongs to the CLI or consumer, which makes secret handling explicit.

## In-memory generation

`PostgresMetadataReader` converts PostgreSQL catalog metadata into `SchemaModel`. A caller can also construct `SchemaModel` directly and use `Generator::render`, `Generator::generate`, or `Generator::verify`.

This separation keeps rule/type/naming/render behavior testable without a live database.

## Errors

Expected user/config/schema failures return `postgres_graphql_schema::Error`. Production generation does not use panics for expected errors.

Database connection errors are sanitized before becoming public errors; the original DSN/password/query values are not intentionally included.

## SemVer

The 0.x series may evolve public Rust API with minor releases. Observable generator/CLI contract changes are documented in CHANGELOG.md and docs/compatibility.md and require compatibility tests.
