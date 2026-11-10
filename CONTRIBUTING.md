# Contributing

Use Rust 1.99.1 as pinned by `rust-toolchain.toml`.

Before pushing:

```bash
make verify-push
```

Before a release or any metadata/naming/type behavior change:

```bash
make verify
```

A new DBMS backend is accepted only with a synthetic integration fixture, deterministic generation tests, compiled CLI coverage, documented type semantics, and an ORM/codegen compatibility gate where applicable.

Never weaken stale-rule, unknown-type, collision, or credential-redaction checks to make a fixture pass. Do not commit credentials, private schemas, customer data, or unrelated generated sources.
