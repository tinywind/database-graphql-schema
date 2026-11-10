# Contributing

Use stable Rust 1.99.1 as pinned by `rust-toolchain.toml`.

Before pushing:

```bash
make verify-push
```

Before a release or a change to metadata/naming/type behavior:

```bash
make verify
```

Changes to generated behavior require tests and, when applicable, an update to `docs/compatibility.md`. Never weaken stale-rule, unknown-type, collision, or credential-redaction checks to make a fixture pass.

Commits should be scoped and should not include credentials, private schemas, customer data, or generated files from unrelated repositories.
