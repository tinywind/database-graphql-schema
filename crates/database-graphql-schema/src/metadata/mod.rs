mod mysql;
mod postgres;
mod sqlite;

use crate::{DatabaseConfig, DatabaseDriver, DatabaseKind, Error, Result, SchemaModel};

#[derive(Clone, Debug, Default)]
pub struct DatabaseMetadataReader;

impl DatabaseMetadataReader {
    pub async fn read(&self, database_url: &str, config: &DatabaseConfig) -> Result<SchemaModel> {
        match resolve_kind(database_url, config.driver)? {
            DatabaseKind::Postgres => {
                postgres::read(database_url, config.schema.as_deref().unwrap_or("public")).await
            }
            DatabaseKind::Mysql => mysql::read(database_url, config.schema.as_deref()).await,
            DatabaseKind::Sqlite => {
                sqlite::read(database_url, config.schema.as_deref().unwrap_or("main")).await
            }
        }
    }
}

pub fn detect_database_kind(database_url: &str) -> Result<DatabaseKind> {
    resolve_kind(database_url, DatabaseDriver::Auto)
}

fn resolve_kind(database_url: &str, driver: DatabaseDriver) -> Result<DatabaseKind> {
    match driver {
        DatabaseDriver::Postgres => Ok(DatabaseKind::Postgres),
        DatabaseDriver::Mysql => Ok(DatabaseKind::Mysql),
        DatabaseDriver::Sqlite => Ok(DatabaseKind::Sqlite),
        DatabaseDriver::Auto => {
            let lowered = database_url.trim_start().to_ascii_lowercase();
            if lowered.starts_with("postgres://") || lowered.starts_with("postgresql://") {
                Ok(DatabaseKind::Postgres)
            } else if lowered.starts_with("mysql://") || lowered.starts_with("mariadb://") {
                Ok(DatabaseKind::Mysql)
            } else if lowered.starts_with("sqlite:") {
                Ok(DatabaseKind::Sqlite)
            } else {
                Err(Error::InvalidConfig(
                    "database.driver=auto cannot infer the DBMS from DATABASE_URL; use postgres/postgresql, mysql/mariadb, or sqlite URL schemes, or set database.driver explicitly".to_owned(),
                ))
            }
        }
    }
}

pub(super) fn non_blank(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_url_schemes() {
        assert_eq!(
            detect_database_kind("postgresql://localhost/db").unwrap(),
            DatabaseKind::Postgres
        );
        assert_eq!(
            detect_database_kind("mysql://localhost/db").unwrap(),
            DatabaseKind::Mysql
        );
        assert_eq!(
            detect_database_kind("mariadb://localhost/db").unwrap(),
            DatabaseKind::Mysql
        );
        assert_eq!(
            detect_database_kind("sqlite::memory:").unwrap(),
            DatabaseKind::Sqlite
        );
        assert!(detect_database_kind("oracle://localhost/db").is_err());
    }
}
