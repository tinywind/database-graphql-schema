#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

image="postgres:18.6"
name="database-graphql-schema-test-$$"
password="fixture-password"
database="graphql_schema_test"

cleanup() {
  docker rm -f "$name" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "$name"   -e POSTGRES_PASSWORD="$password"   -e POSTGRES_DB="$database"   -P "$image" >/dev/null

for attempt in $(seq 1 60); do
  if docker exec "$name" pg_isready -U postgres -d "$database" >/dev/null 2>&1; then
    break
  fi
  if [[ "$attempt" -eq 60 ]]; then
    echo "PostgreSQL 18.6 fixture did not become ready" >&2
    exit 1
  fi
  sleep 1
done

docker exec -i "$name" psql -v ON_ERROR_STOP=1 -U postgres -d "$database"   < fixtures/postgres/fixture.sql >/dev/null

port="$(docker port "$name" 5432/tcp | head -1 | sed 's/.*://')"
export TEST_DATABASE_URL="postgresql://postgres:${password}@127.0.0.1:${port}/${database}"

cargo test -p database-graphql-schema --test postgres_integration -- --ignored
cargo test -p cargo-graphql-schema --test functional -- --ignored
scripts/verify-seaorm.sh

echo "PostgreSQL 18.6 integration/functional/compatibility: PASS"
