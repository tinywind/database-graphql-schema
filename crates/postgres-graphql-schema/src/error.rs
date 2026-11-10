use std::path::PathBuf;

use thiserror::Error;
use url::Url;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read configuration {path}: {message}")]
    ConfigRead { path: PathBuf, message: String },
    #[error("invalid configuration {path}: {message}")]
    ConfigParse { path: PathBuf, message: String },
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("PostgreSQL metadata error: {0}")]
    Database(String),
    #[error("schema generation failed: {0}")]
    Generation(String),
    #[error("failed to write generated SDL to {path}: {message}")]
    OutputWrite { path: PathBuf, message: String },
    #[error("failed to read generated SDL from {path}: {message}")]
    OutputRead { path: PathBuf, message: String },
}

pub(crate) fn database_error(
    database_url: &str,
    context: &str,
    error: impl std::fmt::Display,
) -> Error {
    let mut message = format!("{context}: {error}");
    if !database_url.is_empty() {
        message = message.replace(database_url, "[REDACTED_DATABASE_URL]");
    }
    if let Ok(url) = Url::parse(database_url) {
        if let Some(password) = url.password().filter(|value| !value.is_empty()) {
            message = message.replace(password, "[REDACTED]");
        }
        for (_, value) in url.query_pairs() {
            if !value.is_empty() {
                message = message.replace(value.as_ref(), "[REDACTED]");
            }
        }
    }
    Error::Database(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_errors_redact_password_query_and_full_dsn() {
        let dsn = "postgresql://user:swordfish@localhost:5432/db?sslpassword=deep-secret";
        let error = database_error(
            dsn,
            "connect failed",
            format!("bad password swordfish at {dsn}; sslpassword deep-secret"),
        );
        let rendered = error.to_string();
        assert!(!rendered.contains("swordfish"));
        assert!(!rendered.contains("deep-secret"));
        assert!(!rendered.contains(dsn));
        assert!(rendered.contains("[REDACTED]"));
    }
}
