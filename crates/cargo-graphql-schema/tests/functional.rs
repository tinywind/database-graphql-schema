#![forbid(unsafe_code)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cargo-graphql-schema"))
}

fn database_url() -> String {
    env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must point at the fixture database")
}

fn write_config(directory: &Path, extra: &str) -> PathBuf {
    let output = directory.join("entities.graphqls");
    let config = directory.join("graphql-schema.toml");
    fs::write(
        &config,
        format!(
            r#"
[database]
schema = "public"
url_env = "TEST_DATABASE_URL"

[generator]
output = "{}"
descriptions = true
header = ""

[tables]
include = [".*"]
exclude = ["internal_.*"]

[columns]
exclude_foreign_keys = true
keep_primary_keys = true
include = ["project\\.owner_id"]
exclude = ["account\\.secret_hash", "unsupported_record\\.payload"]

[types]
nullable = []
non_null = []

[types.sql]
jsonb = "String"
numeric = "BigDecimal"

[[types.override]]
pattern = "organization\\.network"
graphql = "String"

[enums]
include_unreferenced = false
include = []
{}
"#,
            output.display(),
            extra
        ),
    )
    .unwrap();
    config
}

fn run(config: &Path, command: &str, url: &str) -> std::process::Output {
    Command::new(binary())
        .arg(command)
        .arg("--config")
        .arg(config)
        .env("TEST_DATABASE_URL", url)
        .output()
        .unwrap()
}

#[test]
#[ignore = "requires TEST_DATABASE_URL with fixtures/postgres/fixture.sql applied"]
fn compiled_cli_generate_verify_and_fail_closed_paths() {
    let temp = tempfile::tempdir().unwrap();
    let config = write_config(temp.path(), "");
    let url = database_url();
    let output_file = temp.path().join("entities.graphqls");

    let generated = run(&config, "generate", &url);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let canonical = fs::read_to_string(&output_file).unwrap();
    assert!(canonical.contains("type Account"));
    assert!(canonical.contains("ownerId: UUID!"));

    let verified = run(&config, "verify", &url);
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );

    fs::write(&output_file, format!("{canonical}\n# modified\n")).unwrap();
    let different = run(&config, "verify", &url);
    assert!(!different.status.success());
    let stderr = String::from_utf8_lossy(&different.stderr);
    assert!(stderr.contains("differs from the PostgreSQL schema at line"));

    fs::remove_file(&output_file).unwrap();
    let missing = run(&config, "verify", &url);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("does not exist"));

    let stale_config = write_config(temp.path(), "");
    let stale_source = fs::read_to_string(&stale_config).unwrap().replace(
        "nullable = []",
        r#"nullable = ["account\\.renamed_column"]"#,
    );
    fs::write(&stale_config, stale_source).unwrap();
    let stale = run(&stale_config, "generate", &url);
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("matches nothing"));

    let wrong_schema_source = fs::read_to_string(&config)
        .unwrap()
        .replace("schema = \"public\"", "schema = \"missing_schema\"");
    fs::write(&config, wrong_schema_source).unwrap();
    let wrong_schema = run(&config, "generate", &url);
    assert!(!wrong_schema.status.success());
    assert!(String::from_utf8_lossy(&wrong_schema.stderr).contains("missing_schema"));
}

#[test]
#[ignore = "network failure smoke test; does not need a live PostgreSQL fixture"]
fn connection_failure_redacts_password() {
    let temp = tempfile::tempdir().unwrap();
    let config = write_config(temp.path(), "");
    let secret = "functional-super-secret";
    let url =
        format!("postgresql://nobody:{secret}@127.0.0.1:1/none?sslmode=disable&connect_timeout=1");
    let failed = run(&config, "generate", &url);
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(!stderr.contains(secret), "{stderr}");
}
