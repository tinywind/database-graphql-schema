# Verify SeaORM compatibility

Use actual SeaORM 2.0.4 DB-first codegen, not a handwritten approximation.

1. Confirm `sea-orm-cli --version` is 2.0.4.
2. Run `scripts/verify-databases.sh`.
3. Verify table/entity naming, Rust fields, native enums, nullability, PK/FK behavior, representative numeric/temporal/JSON/binary types, and PostgreSQL arrays.
4. Treat DBMS-specific catalog behavior as evidence, not a reason to normalize it away.
5. Treat codegen drift as a compatibility finding. Production code must not depend on private sea-orm-codegen APIs.
