#![forbid(unsafe_code)]

use std::{env, path::PathBuf};

use database_graphql_schema::{Config, DatabaseKind, DatabaseMetadataReader, render_from_database};

fn database_url() -> String {
    env::var("TEST_MYSQL_URL")
        .expect("TEST_MYSQL_URL must point at the synthetic MySQL/MariaDB fixture")
}

fn fixture_config() -> Config {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    Config::from_path(root.join("fixtures/mysql/graphql-schema.toml")).unwrap()
}

#[tokio::test]
#[ignore = "requires TEST_MYSQL_URL with fixtures/mysql/fixture.sql applied"]
async fn reads_mysql_or_mariadb_metadata() {
    let config = fixture_config();
    let schema = DatabaseMetadataReader
        .read(&database_url(), &config.database)
        .await
        .unwrap();

    assert_eq!(schema.database_kind, DatabaseKind::Mysql);
    assert!(!schema.schema_name.is_empty());

    let account = schema
        .tables
        .iter()
        .find(|table| table.name == "account")
        .unwrap();
    assert_eq!(account.comment.as_deref(), Some("Account rows."));
    let column = |name: &str| {
        account
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap()
    };
    assert!(column("id").primary_key);
    assert!(column("organization_id").foreign_key);
    assert_eq!(column("balance").numeric_precision, Some(12));
    assert_eq!(column("balance").numeric_scale, Some(2));
    assert_eq!(column("sequence_no").numeric_scale, Some(0));
    assert_eq!(column("status").enum_key.as_deref(), Some("account.status"));

    let status = schema
        .enums
        .iter()
        .find(|value| value.rule_name == "account.status")
        .unwrap();
    assert_eq!(status.graphql_name, "Status");
    assert_eq!(status.values, ["PENDING", "APPROVED", "REJECTED"]);

    let membership = schema
        .tables
        .iter()
        .find(|table| table.name == "membership")
        .unwrap();
    assert!(membership.columns[0].primary_key && membership.columns[0].foreign_key);
    assert!(membership.columns[1].primary_key && membership.columns[1].foreign_key);
}

#[tokio::test]
#[ignore = "requires TEST_MYSQL_URL with fixtures/mysql/fixture.sql applied"]
async fn mysql_or_mariadb_generation_is_deterministic() {
    let config = fixture_config();
    let first = render_from_database(&config, &database_url())
        .await
        .unwrap();
    let second = render_from_database(&config, &database_url())
        .await
        .unwrap();
    assert_eq!(first.as_bytes(), second.as_bytes());

    assert!(first.contains("enum Status"));
    assert_eq!(first.matches("enum Status").count(), 1);
    assert!(first.contains("type Account"));
    assert!(first.contains("status: Status!"));
    assert!(first.contains("sequenceNo: BigDecimal!"));
    assert!(first.contains("active: Int!"));
    assert!(first.contains("avatar: Bytes"));
    assert!(first.contains("ownerId: Long!"));
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
#[ignore = "requires TEST_MYSQL_URL with fixtures/mysql/fixture.sql applied"]
async fn wrong_mysql_schema_fails_without_exposing_database_url() {
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
