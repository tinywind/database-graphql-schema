#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

verify_one() {
  local flavor="$1"
  local image="$2"
  local name="database-graphql-schema-${flavor}-$$"
  local user="fixture"
  local password="fixture-password"
  local database="graphql_schema_test"

  cleanup_one() {
    docker rm -f "$name" >/dev/null 2>&1 || true
  }
  trap cleanup_one RETURN

  if [[ "$flavor" == "mysql" ]]; then
    docker run -d --name "$name"       -e MYSQL_ROOT_PASSWORD=root-fixture-password       -e MYSQL_DATABASE="$database"       -e MYSQL_USER="$user"       -e MYSQL_PASSWORD="$password"       -P "$image" >/dev/null
    for attempt in $(seq 1 120); do
      if docker exec "$name" sh -lc "MYSQL_PWD=$password mysql -u$user -D $database -Nse 'SELECT 1'" >/dev/null 2>&1; then
        break
      fi
      [[ "$attempt" -lt 120 ]] || { docker logs "$name" | tail -120; return 1; }
      sleep 1
    done
    docker exec -i "$name" sh -lc "MYSQL_PWD=$password exec mysql -u$user $database" < fixtures/mysql/fixture.sql
  else
    docker run -d --name "$name"       -e MARIADB_ROOT_PASSWORD=root-fixture-password       -e MARIADB_DATABASE="$database"       -e MARIADB_USER="$user"       -e MARIADB_PASSWORD="$password"       -P "$image" >/dev/null
    for attempt in $(seq 1 120); do
      if docker exec "$name" sh -lc "MYSQL_PWD=$password mariadb -u$user -D $database -Nse 'SELECT 1'" >/dev/null 2>&1; then
        break
      fi
      [[ "$attempt" -lt 120 ]] || { docker logs "$name" | tail -120; return 1; }
      sleep 1
    done
    docker exec -i "$name" sh -lc "MYSQL_PWD=$password exec mariadb -u$user $database" < fixtures/mysql/fixture.sql
  fi

  local port
  port="$(docker port "$name" 3306/tcp | head -1 | sed 's/.*://')"
  export TEST_MYSQL_URL="mysql://${user}:${password}@127.0.0.1:${port}/${database}"

  cargo test -p database-graphql-schema --test mysql_integration -- --ignored
  cargo test -p cargo-graphql-schema --test multi_backend_functional     mysql_or_mariadb_cli_generate_and_verify -- --ignored
  scripts/verify-seaorm.sh "$flavor"

  cleanup_one
  trap - RETURN
  echo "${flavor} compatibility: PASS"
}

verify_one mysql mysql:26.10.0
verify_one mariadb mariadb:13.0.2
