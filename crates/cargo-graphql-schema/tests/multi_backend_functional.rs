#![forbid(unsafe_code)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cargo-graphql-schema"))
}

fn prepared_config(source: &Path, directory: &Path) -> PathBuf {
    let output = directory.join("entities.graphqls");
    let config = directory.join("graphql-schema.toml");
    let text = fs::read_to_string(source).unwrap().replace(
        "output = \"actual.graphqls\"",
        &format!("output = \"{}\"", output.display()),
    );
    fs::write(&config, text).unwrap();
    config
}

fn run(config: &Path, command: &str, env_name: &str, url: &str) -> std::process::Output {
    Command::new(binary())
        .arg(command)
        .arg("--config")
        .arg(config)
        .env(env_name, url)
        .output()
        .unwrap()
}

fn exercise(source: &Path, env_name: &str, url: &str, expected: &[&str]) {
    let temp = tempfile::tempdir().unwrap();
    let config = prepared_config(source, temp.path());
    let output_file = temp.path().join("entities.graphqls");

    let generated = run(&config, "generate", env_name, url);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let content = fs::read_to_string(&output_file).unwrap();
    for value in expected {
        assert!(content.contains(value), "missing {value} in\n{content}");
    }

    let verified = run(&config, "verify", env_name, url);
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );

    fs::write(&output_file, format!("{content}\n# changed\n")).unwrap();
    let different = run(&config, "verify", env_name, url);
    assert!(!different.status.success());
    assert!(
        String::from_utf8_lossy(&different.stderr)
            .contains("differs from the database schema at line")
    );
}

#[test]
#[ignore = "requires TEST_MYSQL_URL with fixtures/mysql/fixture.sql applied"]
fn mysql_or_mariadb_cli_generate_and_verify() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let url = env::var("TEST_MYSQL_URL").expect("TEST_MYSQL_URL");
    exercise(
        &root.join("fixtures/mysql/graphql-schema.toml"),
        "TEST_MYSQL_URL",
        &url,
        &["enum Status", "type Account", "ownerId: Long!"],
    );
}

#[test]
#[ignore = "requires TEST_SQLITE_URL with fixtures/sqlite/fixture.sql applied"]
fn sqlite_cli_generate_and_verify() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let url = env::var("TEST_SQLITE_URL").expect("TEST_SQLITE_URL");
    exercise(
        &root.join("fixtures/sqlite/graphql-schema.toml"),
        "TEST_SQLITE_URL",
        &url,
        &["type Account", "id: Long!", "ownerId: Long!"],
    );
}
