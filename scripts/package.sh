#!/usr/bin/env bash
set -euo pipefail

version="$(python3 - <<'PY'
import tomllib
with open("Cargo.toml", "rb") as f:
    print(tomllib.load(f)["workspace"]["package"]["version"])
PY
)"

cargo package -p database-graphql-schema --allow-dirty

if (cd /tmp && cargo info "database-graphql-schema@$version" >/dev/null 2>&1); then
  cargo package -p cargo-graphql-schema --allow-dirty
else
  echo "cargo-graphql-schema package verification deferred until database-graphql-schema $version exists in crates.io (first-release dependency ordering)."
fi
