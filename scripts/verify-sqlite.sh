#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
db="$tmp/fixture.sqlite"

python3 - "$db" fixtures/sqlite/fixture.sql <<'PY'
import sqlite3
import sys

db, fixture = sys.argv[1], sys.argv[2]
connection = sqlite3.connect(db)
connection.executescript(open(fixture, encoding="utf-8").read())
print(f"SQLite fixture writer: {sqlite3.sqlite_version}")
connection.close()
PY

export TEST_SQLITE_URL="sqlite://$db"
cargo test -p database-graphql-schema --test sqlite_integration -- --ignored
cargo test -p cargo-graphql-schema --test multi_backend_functional   sqlite_cli_generate_and_verify -- --ignored
scripts/verify-seaorm.sh sqlite

echo "SQLite compatibility: PASS"
