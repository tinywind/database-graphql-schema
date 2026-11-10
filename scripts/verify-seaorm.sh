#!/usr/bin/env bash
set -euo pipefail

backend="${1:-postgres}"
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
mkdir -p "$tmp/entities"

case "$backend" in
  postgres)
    : "${TEST_DATABASE_URL:?TEST_DATABASE_URL is required}"
    DATABASE_URL="$TEST_DATABASE_URL" sea-orm-cli generate entity       --database-schema public       --output-dir "$tmp/entities"       --with-prelude none       --banner-version patch >/dev/null
    TEST_DATABASE_URL="$TEST_DATABASE_URL" cargo run --quiet -p cargo-graphql-schema --       print --config "$repo_root/fixtures/java-0.1.1/graphql-schema.toml" > "$tmp/entities.graphqls"
    ;;
  mysql | mariadb)
    : "${TEST_MYSQL_URL:?TEST_MYSQL_URL is required}"
    DATABASE_URL="$TEST_MYSQL_URL" sea-orm-cli generate entity       --output-dir "$tmp/entities"       --with-prelude none       --banner-version patch >/dev/null
    TEST_MYSQL_URL="$TEST_MYSQL_URL" cargo run --quiet -p cargo-graphql-schema --       print --config "$repo_root/fixtures/mysql/graphql-schema.toml" > "$tmp/entities.graphqls"
    ;;
  sqlite)
    : "${TEST_SQLITE_URL:?TEST_SQLITE_URL is required}"
    DATABASE_URL="$TEST_SQLITE_URL" sea-orm-cli generate entity       --output-dir "$tmp/entities"       --with-prelude none       --banner-version patch >/dev/null
    TEST_SQLITE_URL="$TEST_SQLITE_URL" cargo run --quiet -p cargo-graphql-schema --       print --config "$repo_root/fixtures/sqlite/graphql-schema.toml" > "$tmp/entities.graphqls"
    ;;
  *)
    echo "usage: scripts/verify-seaorm.sh postgres|mysql|sqlite" >&2
    exit 2
    ;;
esac

python3 - "$backend" "$tmp/entities" "$tmp/entities.graphqls" <<'PY'
from pathlib import Path
import sys

backend = sys.argv[1]
entities = Path(sys.argv[2])
sdl = Path(sys.argv[3]).read_text()

def read(name: str) -> str:
    path = entities / name
    if not path.is_file():
        raise SystemExit(f"SeaORM compatibility ({backend}): missing generated {name}")
    return path.read_text()

def need(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"SeaORM compatibility ({backend}): missing {label}: {needle}")

account = read("account.rs")
membership = read("membership.rs")
project = read("project.rs")
read("organization.rs")
read("unsupported_record.rs")

if backend == "postgres":
    enums = read("sea_orm_active_enums.rs")
    for needle, label in [
        ("pub id: Uuid,", "UUID"),
        ("pub organization_id: Option<Uuid>,", "nullable FK UUID"),
        ("pub status: ReviewStatus,", "enum"),
        ("pub tags: Option<Vec<String>>,", "array"),
        ("pub matrix: Option<Vec<i32>>,", "SeaORM 2.0.4 multidimensional-array behavior"),
        ("pub profile: Option<Json>,", "JSONB"),
        ("pub balance: Option<Decimal>,", "numeric Decimal"),
        ("pub sequence_no: Decimal,", "zero-scale numeric Decimal"),
        ("pub remote_created: DateTimeWithTimeZone,", "timestamptz"),
    ]:
        need(account, needle, label)
    for needle in [
        'enum_name = "review_status"',
        'string_value = "PENDING"',
        'string_value = "APPROVED"',
        'string_value = "REJECTED"',
    ]:
        need(enums, needle, "PostgreSQL enum identity")
    for needle in [
        "enum ReviewStatus",
        "id: UUID!",
        "status: ReviewStatus!",
        "tags: [String!]",
        "matrix: [[Int!]!]",
        "sequenceNo: BigDecimal!",
        "ownerId: UUID!",
    ]:
        need(sdl, needle, "GraphQL PostgreSQL shape")

elif backend in ("mysql", "mariadb"):
    enums = read("sea_orm_active_enums.rs")
    if backend == "mysql":
        need(account, "pub profile: Option<Json>,", "MySQL JSON")
    else:
        need(account, "pub profile: Option<String>,", "MariaDB JSON alias / LONGTEXT")
    for needle, label in [
        ("pub id: i64,", "BIGINT"),
        ("pub organization_id: Option<i64>,", "nullable FK BIGINT"),
        ("pub status: Status,", "inline ENUM"),
        ("pub balance: Option<Decimal>,", "numeric Decimal"),
        ("pub sequence_no: Decimal,", "zero-scale numeric Decimal"),
        ("pub remote_created: Option<DateTimeUtc>,", "timestamp"),
        ("pub active: i8,", "MySQL BOOLEAN/TINYINT alias"),
        ("pub avatar: Option<Vec<u8>>,", "BLOB"),
    ]:
        need(account, needle, label)
    for needle in [
        'enum_name = "status"',
        'string_value = "PENDING"',
        'string_value = "APPROVED"',
        'string_value = "REJECTED"',
    ]:
        need(enums, needle, "MySQL enum identity")
    for needle in [
        "enum Status",
        "id: Long!",
        "status: Status!",
        "sequenceNo: BigDecimal!",
        "active: Int!",
        "avatar: Bytes",
        "ownerId: Long!",
    ]:
        need(sdl, needle, "GraphQL MySQL shape")

elif backend == "sqlite":
    for needle, label in [
        ("pub id: i64,", "INTEGER"),
        ("pub organization_id: Option<i64>,", "nullable FK INTEGER"),
        ("pub profile: Option<Json>,", "JSON_TEXT"),
        ("pub balance: Option<Decimal>,", "Decimal"),
        ("pub sequence_no: Decimal,", "zero-scale Decimal"),
        ("pub birthday: Option<Date>,", "DATE_TEXT"),
        ("pub local_created: Option<DateTime>,", "DATETIME_TEXT"),
        ("pub remote_created: Option<DateTimeWithTimeZone>,", "timestamp-with-time-zone text"),
        ("pub active: bool,", "BOOLEAN"),
        ("pub avatar: Option<Vec<u8>>,", "BLOB"),
    ]:
        need(account, needle, label)
    for needle in [
        "id: Long!",
        "profile: JSON",
        "sequenceNo: BigDecimal!",
        "active: Boolean!",
        "avatar: Bytes",
        "ownerId: Long!",
    ]:
        need(sdl, needle, "GraphQL SQLite shape")

for needle in ["pub account_id:", "pub organization_id:"]:
    need(membership, needle, "composite PK/FK field")
need(project, "pub owner_id:", "explicitly exposed FK field")

account_block = sdl.split("type Account {", 1)[1].split("\n}", 1)[0]
if "organizationId" in account_block or "secretHash" in account_block:
    raise SystemExit(f"SeaORM compatibility ({backend}): GraphQL exposure filters drifted")
if "type InternalAudit" in sdl:
    raise SystemExit(f"SeaORM compatibility ({backend}): table exclusion drifted")

print(f"SeaORM 2.0.4 compatibility ({backend}): PASS")
PY
