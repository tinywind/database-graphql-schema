# Verify SeaORM compatibility

Use the actual pinned SeaORM CLI, never a handwritten approximation.

1. Confirm `sea-orm-cli --version` is 2.0.4.
2. Run `scripts/verify-postgres.sh`, which generates SeaORM entities from the same synthetic PostgreSQL fixture used by this project.
3. Verify table/entity naming, Rust column fields, enum database names/values, nullability, PK/FK behavior, arrays, UUID, numeric/Decimal, temporal, JSON, and INET expectations.
4. Treat any codegen drift as a compatibility finding. Do not add sea-orm-codegen as a production dependency merely to make the gate pass.
