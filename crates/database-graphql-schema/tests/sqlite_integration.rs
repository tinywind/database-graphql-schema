#![forbid(unsafe_code)]

use std::{env, path::PathBuf};

use database_graphql_schema::{Config, DatabaseKind, DatabaseMetadataReader, render_from_database};

fn database_url() -> String {
    env::var("TEST_SQLITE_URL").expect("TEST_SQLITE_URL must point at the synthetic SQLite fixture")
}

fn fixture_config() -> Config {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    Config::from_path(root.join("fixtures/sqlite/graphql-schema.toml")).unwrap()
}

#[tokio::test]
#[ignore = "requires TEST_SQLITE_URL with fixtures/sqlite/fixture.sql applied"]
async fn reads_sqlite_metadata() {
    let config = fixture_config();
    let schema = DatabaseMetadataReader
        .read(&database_url(), &config.database)
        .await
        .unwrap();

    assert_eq!(schema.database_kind, DatabaseKind::Sqlite);
    assert_eq!(schema.schema_name, "main");
    assert!(schema.enums.is_empty());

    let account = schema
        .tables
        .iter()
        .find(|table| table.name == "account")
        .unwrap();
    assert!(account.comment.is_none());
    let column = |name: &str| {
        account
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap()
    };
    assert!(column("id").primary_key);
    assert!(column("organization_id").foreign_key);
    assert_eq!(column("id").type_name, "integer");
    assert_eq!(column("profile").type_name, "json");
    assert_eq!(column("balance").numeric_precision, Some(12));
    assert_eq!(column("balance").numeric_scale, Some(2));

    let membership = schema
        .tables
        .iter()
        .find(|table| table.name == "membership")
        .unwrap();
    assert!(membership.columns[0].primary_key && membership.columns[0].foreign_key);
    assert!(membership.columns[1].primary_key && membership.columns[1].foreign_key);
}

#[tokio::test]
#[ignore = "requires TEST_SQLITE_URL with fixtures/sqlite/fixture.sql applied"]
async fn sqlite_generation_is_deterministic() {
    let config = fixture_config();
    let first = render_from_database(&config, &database_url())
        .await
        .unwrap();
    let second = render_from_database(&config, &database_url())
        .await
        .unwrap();
    assert_eq!(first.as_bytes(), second.as_bytes());

    assert!(first.contains("type Account"));
    assert!(first.contains("id: Long!"));
    assert!(first.contains("profile: JSON"));
    assert!(first.contains("sequenceNo: BigDecimal!"));
    assert!(first.contains("active: Boolean!"));
    assert!(first.contains("avatar: Bytes"));
    assert!(first.contains("ownerId: Long!"));
    assert!(!first.contains("enum "));
    assert!(!first.contains("type InternalAudit"));
    assert!(!first.contains("payload:"));

    let account_block = first
        .split("type Account {")
        .nth(1)
        .unwrap()
        .split("\n}")
        .next()
        .unwrap();
    assert!(!account_block.contains("organizationId"));
    assert!(!account_block.contains("secretHash"));
}

#[tokio::test]
#[ignore = "requires TEST_SQLITE_URL with fixtures/sqlite/fixture.sql applied"]
async fn wrong_sqlite_schema_fails_without_exposing_database_url() {
    let url = database_url();
    let mut config = fixture_config();
    config.database.schema = Some("definitely_missing".to_owned());
    let error = DatabaseMetadataReader
        .read(&url, &config.database)
        .await
        .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("definitely_missing"));
    assert!(!message.contains(&url));
}
