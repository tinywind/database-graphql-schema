#!/usr/bin/env bash
set -euo pipefail

: "${TEST_DATABASE_URL:?TEST_DATABASE_URL is required}"
command -v sea-orm-cli >/dev/null || {
  echo "sea-orm-cli 2.0.4 is required for compatibility verification" >&2
  exit 1
}

version="$(sea-orm-cli --version)"
[[ "$version" == "sea-orm-cli 2.0.4" ]] || {
  echo "expected sea-orm-cli 2.0.4, got: $version" >&2
  exit 1
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

DATABASE_URL="$TEST_DATABASE_URL" sea-orm-cli generate entity   --database-schema public   --output-dir "$tmp/entities"   --with-prelude none   --banner-version patch >/dev/null

TEST_DATABASE_URL="$TEST_DATABASE_URL" cargo run --quiet -p cargo-graphql-schema --   print --config "$repo_root/fixtures/java-0.1.1/graphql-schema.toml" > "$tmp/entities.graphqls"

python3 - "$tmp/entities" "$tmp/entities.graphqls" <<'PY'
from pathlib import Path
import re
import sys

entities = Path(sys.argv[1])
sdl = Path(sys.argv[2]).read_text()

def read(name: str) -> str:
    path = entities / name
    if not path.is_file():
        raise SystemExit(f"SeaORM compatibility: missing generated {name}")
    return path.read_text()

account = read("account.rs")
membership = read("membership.rs")
project = read("project.rs")
enums = read("sea_orm_active_enums.rs")
for name in ("organization.rs", "unsupported_record.rs"):
    read(name)

def need(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"SeaORM compatibility: missing {label}: {needle}")

# Actual SeaORM 2.0.4 model types for the common DB-backed columns.
for needle, label in [
    ("pub id: Uuid,", "UUID"),
    ("pub organization_id: Option<Uuid>,", "nullable FK UUID"),
    ("pub status: ReviewStatus,", "enum"),
    ("pub tags: Option<Vec<String>>,", "array"),
    ("pub matrix: Option<Vec<i32>>,", "SeaORM 2.0.4 multidimensional-array behavior"),
    ("pub profile: Option<Json>,", "JSONB"),
    ("pub balance: Option<Decimal>,", "numeric(scale>0) Decimal"),
    ("pub sequence_no: Decimal,", "numeric(scale=0) Decimal"),
    ("pub birthday: Option<Date>,", "date"),
    ("pub local_time: Option<Time>,", "time"),
    ("pub offset_time: Option<Time>,", "timetz"),
    ("pub local_created: Option<DateTime>,", "timestamp"),
    ("pub remote_created: DateTimeWithTimeZone,", "timestamptz"),
]:
    need(account, needle, label)

need(project, "pub owner_id: Uuid,", "explicitly exposed FK field")
need(membership, "pub account_id: Uuid,", "composite PK/FK account")
need(membership, "pub organization_id: Uuid,", "composite PK/FK organization")
if membership.count("#[sea_orm(primary_key, auto_increment = false)]") != 2:
    raise SystemExit("SeaORM compatibility: composite PK markers drifted")

for needle in [
    'enum_name = "review_status"',
    'string_value = "PENDING"',
    'string_value = "APPROVED"',
    'string_value = "REJECTED"',
]:
    need(enums, needle, "PostgreSQL enum identity")

# GraphQL names and types derived from the same DB schema.
for needle in [
    "enum ReviewStatus",
    "type Account",
    "    id: UUID!",
    "    status: ReviewStatus!",
    "    tags: [String!]",
    "    matrix: [[Int!]!]",
    "    balance: BigDecimal",
    "    sequenceNo: BigDecimal!",
    "    birthday: Date",
    "    localTime: LocalTime",
    "    offsetTime: Time",
    "    localCreated: LocalDateTime",
    "    remoteCreated: DateTime!",
    "type Membership",
    "    accountId: UUID!",
    "    organizationId: UUID!",
    "type Project",
    "    ownerId: UUID!",
]:
    need(sdl, needle, "GraphQL DB-backed shape")

account_block = sdl.split("type Account {", 1)[1].split("\n}", 1)[0]
if "organizationId" in account_block or "secretHash" in account_block:
    raise SystemExit("SeaORM compatibility: GraphQL exposure filters drifted")
if "type InternalAudit" in sdl:
    raise SystemExit("SeaORM compatibility: table exclusion drifted")

# PostgreSQL has attndims=2 for matrix; SeaORM 2.0.4 currently emits Vec<i32>.
# The GraphQL generator intentionally preserves DB dimensions. Keep this explicit
# so an upstream SeaORM fix becomes a compatibility-review event.
need(account, "pub matrix: Option<Vec<i32>>,", "documented SeaORM multidimensional-array limitation")
need(sdl, "matrix: [[Int!]!]", "PostgreSQL multidimensional-array fidelity")

print("SeaORM 2.0.4 compatibility: PASS")
PY
