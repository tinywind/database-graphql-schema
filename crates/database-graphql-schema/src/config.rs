use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    #[default]
    Auto,
    #[serde(alias = "postgresql")]
    Postgres,
    #[serde(alias = "mariadb")]
    Mysql,
    Sqlite,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub database: DatabaseConfig,
    pub generator: GeneratorConfig,
    #[serde(default)]
    pub tables: TableConfig,
    #[serde(default)]
    pub columns: ColumnsConfig,
    #[serde(default)]
    pub types: TypesConfig,
    #[serde(default)]
    pub enums: EnumsConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DatabaseConfig {
    pub driver: DatabaseDriver,
    pub schema: Option<String>,
    pub url_env: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            driver: DatabaseDriver::Auto,
            schema: None,
            url_env: "DATABASE_URL".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorConfig {
    pub output: PathBuf,
    #[serde(default = "default_true")]
    pub descriptions: bool,
    #[serde(default)]
    pub header: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TableConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ColumnsConfig {
    pub exclude_foreign_keys: bool,
    pub keep_primary_keys: bool,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for ColumnsConfig {
    fn default() -> Self {
        Self {
            exclude_foreign_keys: true,
            keep_primary_keys: true,
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TypesConfig {
    pub sql: BTreeMap<String, String>,
    #[serde(rename = "override")]
    pub overrides: Vec<TypeOverrideConfig>,
    pub nullable: Vec<String>,
    pub non_null: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeOverrideConfig {
    pub pattern: String,
    pub graphql: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnumsConfig {
    pub include_unreferenced: bool,
    pub include: Vec<String>,
}

impl Config {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|error| Error::ConfigRead {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        let mut config: Self = toml::from_str(&source).map_err(|error| Error::ConfigParse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        config.validate()?;
        if config.generator.output.is_relative() {
            let base = path.parent().unwrap_or_else(|| Path::new("."));
            config.generator.output = base.join(&config.generator.output);
        }
        Ok(config)
    }

    pub fn from_toml(source: &str) -> Result<Self> {
        let mut config: Self = toml::from_str(source).map_err(|error| Error::ConfigParse {
            path: PathBuf::from("<memory>"),
            message: error.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&mut self) -> Result<()> {
        self.database.schema = self
            .database
            .schema
            .take()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        self.database.url_env = self.database.url_env.trim().to_owned();
        if self.database.url_env.is_empty() {
            return Err(Error::InvalidConfig(
                "database.url_env must not be blank".to_owned(),
            ));
        }
        if self.generator.output.as_os_str().is_empty() {
            return Err(Error::InvalidConfig(
                "generator.output must not be blank".to_owned(),
            ));
        }
        for rule in &self.types.overrides {
            if rule.graphql.trim().is_empty() {
                return Err(Error::InvalidConfig(format!(
                    "types.override pattern '{}' has a blank GraphQL type",
                    rule.pattern
                )));
            }
        }
        for (sql, graphql) in &self.types.sql {
            if sql.trim().is_empty() || graphql.trim().is_empty() {
                return Err(Error::InvalidConfig(
                    "types.sql entries require non-blank SQL and GraphQL type names".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_toml_with_database_auto_detection() {
        let config = Config::from_toml(
            r#"
            [generator]
            output = "entities.graphqls"

            [tables]
            include = [".*"]
            "#,
        )
        .unwrap();
        assert_eq!(config.database.driver, DatabaseDriver::Auto);
        assert_eq!(config.database.schema, None);
        assert_eq!(config.database.url_env, "DATABASE_URL");
        assert!(config.columns.exclude_foreign_keys);
        assert!(config.columns.keep_primary_keys);
        assert!(config.generator.descriptions);
    }

    #[test]
    fn accepts_driver_aliases() {
        let postgres = Config::from_toml(
            r#"
            [database]
            driver = "postgresql"
            [generator]
            output = "out.graphqls"
            [tables]
            include = [".*"]
            "#,
        )
        .unwrap();
        assert_eq!(postgres.database.driver, DatabaseDriver::Postgres);

        let maria = Config::from_toml(
            r#"
            [database]
            driver = "mariadb"
            [generator]
            output = "out.graphqls"
            [tables]
            include = [".*"]
            "#,
        )
        .unwrap();
        assert_eq!(maria.database.driver, DatabaseDriver::Mysql);
    }
}
