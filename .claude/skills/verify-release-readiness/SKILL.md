# Verify release readiness

A release is ready only after:

- `make verify` passes;
- PostgreSQL, MySQL, MariaDB and SQLite fixtures pass;
- actual SeaORM 2.0.4 compatibility passes for every supported backend;
- `cargo package -p database-graphql-schema` and publish dry-run pass;
- the CLI package/dry-run passes once its exact core dependency is resolvable from crates.io;
- a clean checkout repeats the complete gate;
- README, CHANGELOG, license/NOTICE, version, tag and release notes agree;
- no private schema, credential or internal URL is tracked;
- published crates are consumed from crates.io by a temporary external project.

Do not substitute a workspace path dependency for the published-consumer test.
