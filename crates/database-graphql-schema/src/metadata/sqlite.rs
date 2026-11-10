use std::{collections::HashSet, path::PathBuf};

use rusqlite::{Connection, params};

use crate::{
    ColumnModel, DatabaseKind, Error, Result, SchemaModel, TableModel, error::database_error,
};

pub(super) async fn read(database_url: &str, schema: &str) -> Result<SchemaModel> {
    let database_url = database_url.to_owned();
    let schema = schema.to_owned();
    tokio::task::spawn_blocking(move || read_sync(&database_url, &schema))
        .await
        .map_err(|error| Error::Database(format!("SQLite metadata task failed: {error}")))?
}

fn read_sync(database_url: &str, schema: &str) -> Result<SchemaModel> {
    if schema != "main" {
        return Err(Error::Database(format!(
            "SQLite schema '{schema}' is not attached; the 0.2.x adapter introspects the primary 'main' database"
        )));
    }

    let connection = open(database_url)
        .map_err(|error| database_error(database_url, "SQLite connection failed", error))?;

    let mut table_statement = connection
        .prepare(
            "SELECT name FROM main.sqlite_schema              WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(db_error("preparing SQLite table query"))?;
    let table_names = table_statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(db_error("reading SQLite tables"))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error("decoding SQLite table name"))?;

    let mut tables = Vec::with_capacity(table_names.len());
    for name in table_names {
        tables.push(TableModel {
            columns: read_columns(&connection, &name)?,
            name,
            comment: None,
        });
    }

    Ok(SchemaModel {
        database_kind: DatabaseKind::Sqlite,
        schema_name: "main".to_owned(),
        enums: Vec::new(),
        tables,
    })
}

fn open(database_url: &str) -> std::result::Result<Connection, rusqlite::Error> {
    if database_url.eq_ignore_ascii_case("sqlite::memory:")
        || database_url.eq_ignore_ascii_case("sqlite://:memory:")
    {
        return Connection::open_in_memory();
    }
    let path = sqlite_path(database_url)
        .ok_or(rusqlite::Error::InvalidPath(PathBuf::from(database_url)))?;
    Connection::open(path)
}

fn sqlite_path(database_url: &str) -> Option<PathBuf> {
    let raw = database_url
        .strip_prefix("sqlite://")
        .or_else(|| database_url.strip_prefix("sqlite:"))?;
    let path = raw.split('?').next().unwrap_or(raw);
    if path.is_empty() || path == ":memory:" {
        return None;
    }
    Some(PathBuf::from(path))
}

fn read_columns(connection: &Connection, table: &str) -> Result<Vec<ColumnModel>> {
    let foreign_keys = {
        let mut statement = connection
            .prepare(r#"SELECT "from" FROM pragma_foreign_key_list(?1)"#)
            .map_err(db_error("preparing SQLite foreign-key query"))?;
        statement
            .query_map(params![table], |row| row.get::<_, String>(0))
            .map_err(db_error("reading SQLite foreign keys"))?
            .collect::<std::result::Result<HashSet<_>, _>>()
            .map_err(db_error("decoding SQLite foreign-key column"))?
    };

    let mut statement = connection
        .prepare(
            r#"SELECT name, type, "notnull", pk, hidden
               FROM pragma_table_xinfo(?1)
               ORDER BY cid"#,
        )
        .map_err(db_error("preparing SQLite column query"))?;

    let rows = statement
        .query_map(params![table], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(db_error("reading SQLite columns"))?;

    let mut columns = Vec::new();
    for row in rows {
        let (name, declared, not_null, primary_position, hidden) =
            row.map_err(db_error("decoding SQLite column"))?;
        if hidden == 1 {
            continue;
        }
        let (precision, scale) = numeric_shape(&declared);
        columns.push(ColumnModel {
            name: name.clone(),
            type_name: sqlite_type_name(&declared),
            formatted_type: if declared.trim().is_empty() {
                "<no declared type>".to_owned()
            } else {
                declared
            },
            enum_key: None,
            element_type_name: None,
            element_enum_key: None,
            array_dimensions: 0,
            nullable: not_null == 0 && primary_position == 0,
            primary_key: primary_position > 0,
            foreign_key: foreign_keys.contains(&name),
            comment: None,
            numeric_precision: precision,
            numeric_scale: scale,
        });
    }
    Ok(columns)
}

pub(crate) fn sqlite_type_name(declared: &str) -> String {
    let lowered = declared.trim().to_ascii_lowercase();
    let base = lowered.split('(').next().unwrap_or("").trim();
    if base.is_empty() {
        return "blob".to_owned();
    }

    match base {
        "integer" | "bigint" => "integer".to_owned(),
        "tinyint" => "tinyint".to_owned(),
        "smallint" => "smallint".to_owned(),
        "char" | "varchar" | "text" => base.to_owned(),
        "float" | "double" => base.to_owned(),
        "real" => "real".to_owned(),
        "decimal" | "real_decimal" => "decimal".to_owned(),
        "datetime_text" => "datetime_text".to_owned(),
        "timestamp" | "timestamp_text" => "timestamp".to_owned(),
        "timestamp_with_timezone_text" => "timestamp_with_timezone_text".to_owned(),
        "time_text" => "time_text".to_owned(),
        "date_text" => "date_text".to_owned(),
        "blob" | "varbinary_blob" => "blob".to_owned(),
        "boolean" => "boolean".to_owned(),
        "money" | "real_money" => "decimal".to_owned(),
        "json_text" | "jsonb_text" => "json".to_owned(),
        "uuid_text" => "uuid".to_owned(),
        other if other.contains("int") => "integer".to_owned(),
        other if other.contains("char") || other.contains("clob") || other.contains("text") => {
            "text".to_owned()
        }
        other if other.contains("real") || other.contains("floa") || other.contains("doub") => {
            "double".to_owned()
        }
        other => other.to_owned(),
    }
}

fn numeric_shape(declared: &str) -> (Option<i32>, Option<i32>) {
    let Some((_, suffix)) = declared.split_once('(') else {
        return (None, None);
    };
    let Some(inner) = suffix.strip_suffix(')') else {
        return (None, None);
    };
    let mut parts = inner.split(',').map(str::trim);
    let precision = parts.next().and_then(|value| value.parse().ok());
    let scale = parts.next().and_then(|value| value.parse().ok());
    (precision, scale)
}

fn db_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |error| Error::Database(format!("{context} failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sqlite_is_patched_3_54_1() {
        assert_eq!(rusqlite::version(), "3.54.1");
    }

    #[test]
    fn sqlite_type_normalization_matches_sea_schema_rules() {
        for (declared, expected) in [
            ("INTEGER", "integer"),
            ("BIGINT", "integer"),
            ("VARCHAR(255)", "varchar"),
            ("DOUBLE", "double"),
            ("DECIMAL(12,2)", "decimal"),
            ("UUID_TEXT", "uuid"),
            ("JSON_TEXT", "json"),
            ("custom text flavor", "text"),
            ("", "blob"),
        ] {
            assert_eq!(sqlite_type_name(declared), expected, "{declared}");
        }
    }

    #[test]
    fn parses_numeric_shape() {
        assert_eq!(numeric_shape("decimal(12, 2)"), (Some(12), Some(2)));
        assert_eq!(numeric_shape("integer"), (None, None));
    }

    #[test]
    fn parses_sqlite_urls() {
        assert_eq!(
            sqlite_path("sqlite:///tmp/example.sqlite"),
            Some(PathBuf::from("/tmp/example.sqlite"))
        );
        assert_eq!(
            sqlite_path("sqlite:relative.sqlite?mode=rwc"),
            Some(PathBuf::from("relative.sqlite"))
        );
        assert_eq!(sqlite_path("postgresql://localhost/db"), None);
    }
}
