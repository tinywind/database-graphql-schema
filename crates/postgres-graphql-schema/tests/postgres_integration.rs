#![forbid(unsafe_code)]

use std::{env, path::PathBuf};

use postgres_graphql_schema::{Config, PostgresMetadataReader, render_from_database};

fn database_url() -> String {
    env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point at the synthetic fixture database")
}

fn fixture_config() -> Config {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    Config::from_path(root.join("fixtures/java-0.1.1/graphql-schema.toml")).unwrap()
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with fixtures/postgres/fixture.sql applied"]
async fn reads_real_postgres_metadata() {
    let schema = PostgresMetadataReader
        .read(&database_url(), "public")
        .await
        .unwrap();

    assert_eq!(schema.schema_name, "public");
    let enum_names: Vec<_> = schema
        .enums
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    assert_eq!(enum_names, ["review_status", "unused_state"]);
    let review = schema
        .enums
        .iter()
        .find(|value| value.name == "review_status")
        .unwrap();
    assert_eq!(review.values, ["PENDING", "APPROVED", "REJECTED"]);
    assert_eq!(review.comment.as_deref(), Some("Review workflow state."));

    let account = schema
        .tables
        .iter()
        .find(|table| table.name == "account")
        .unwrap();
    assert!(account.comment.as_deref().unwrap().contains("second line"));
    let column = |name: &str| {
        account
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap()
    };
    assert!(column("id").primary_key);
    assert!(column("organization_id").foreign_key);
    assert_eq!(column("tags").array_dimensions, 1);
    assert_eq!(column("tags").element_type_name.as_deref(), Some("text"));
    assert_eq!(column("matrix").array_dimensions, 2);
    assert_eq!(column("balance").numeric_precision, Some(12));
    assert_eq!(column("balance").numeric_scale, Some(2));
    assert_eq!(column("sequence_no").numeric_precision, Some(14));
    assert_eq!(column("sequence_no").numeric_scale, Some(0));
    assert_eq!(column("profile").type_name, "jsonb");
    assert_eq!(column("remote_created").type_name, "timestamptz");

    let membership = schema
        .tables
        .iter()
        .find(|table| table.name == "membership")
        .unwrap();
    assert!(membership.columns[0].primary_key && membership.columns[0].foreign_key);
    assert!(membership.columns[1].primary_key && membership.columns[1].foreign_key);

    let unsupported = schema
        .tables
        .iter()
        .find(|table| table.name == "unsupported_record")
        .unwrap();
    assert_eq!(unsupported.columns[1].type_name, "unsupported_code");
    assert_eq!(unsupported.columns[1].type_kind, "d");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with fixtures/postgres/fixture.sql applied"]
async fn generation_is_deterministic_on_real_postgres() {
    let config = fixture_config();
    let first = render_from_database(&config, &database_url())
        .await
        .unwrap();
    let second = render_from_database(&config, &database_url())
        .await
        .unwrap();
    assert_eq!(first.as_bytes(), second.as_bytes());

    assert!(first.contains("enum ReviewStatus"));
    assert!(!first.contains("enum UnusedState"));
    assert!(first.contains("type Account"));
    let account_block = first
        .split("type Account {")
        .nth(1)
        .expect("Account type")
        .split(
            "
}",
        )
        .next()
        .expect("Account body");
    assert!(!account_block.contains("organizationId"));
    assert!(first.contains("ownerId: UUID!"));
    assert!(first.contains("matrix: [[Int!]!]"));
    assert!(first.contains("sequenceNo: BigDecimal!"));
    assert!(first.contains("profile: String"));
    assert!(first.contains("type UnsupportedRecord"));
    assert!(!first.contains("payload:"));
    assert!(!first.contains("type InternalAudit"));
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with fixtures/postgres/fixture.sql applied"]
async fn matches_legacy_0_1_1_golden_byte_for_byte() {
    let config = fixture_config();
    let actual = render_from_database(&config, &database_url())
        .await
        .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let expected =
        std::fs::read_to_string(root.join("fixtures/java-0.1.1/expected.graphqls")).unwrap();
    assert_eq!(actual, expected);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with fixtures/postgres/fixture.sql applied"]
async fn wrong_schema_fails_without_exposing_database_url() {
    let url = database_url();
    let error = PostgresMetadataReader
        .read(&url, "definitely_missing")
        .await
        .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("definitely_missing"));
    assert!(!message.contains(&url));
}
