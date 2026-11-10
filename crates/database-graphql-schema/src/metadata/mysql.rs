use std::{collections::BTreeMap, time::Duration};

use sqlx::{Row, mysql::MySqlPoolOptions};

use crate::{
    ColumnModel, DatabaseKind, EnumModel, Error, Result, SchemaModel, TableModel,
    error::database_error, naming::type_name,
};

use super::non_blank;

pub(super) async fn read(
    database_url: &str,
    configured_schema: Option<&str>,
) -> Result<SchemaModel> {
    let normalized_url = if let Some(rest) = database_url.strip_prefix("mariadb://") {
        format!("mysql://{rest}")
    } else {
        database_url.to_owned()
    };

    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&normalized_url)
        .await
        .map_err(|error| database_error(database_url, "MySQL/MariaDB connection failed", error))?;

    let result = async {
        let schema = match configured_schema {
            Some(schema) => schema.to_owned(),
            None => sqlx::query_scalar::<_, Option<String>>("SELECT DATABASE()")
                .fetch_one(&pool)
                .await
                .map_err(|error| Error::Database(format!("reading current MySQL/MariaDB database failed: {error}")))?
                .ok_or_else(|| Error::Database(
                    "MySQL/MariaDB connection has no current database; put the database in DATABASE_URL or set database.schema".to_owned(),
                ))?,
        };

        let schema_exists: i64 = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = ?)",
        )
        .bind(&schema)
        .fetch_one(&pool)
        .await
        .map_err(|error| Error::Database(format!("checking MySQL/MariaDB schema visibility failed: {error}")))?;
        if schema_exists == 0 {
            return Err(Error::Database(format!(
                "MySQL/MariaDB database '{schema}' is not visible to the database connection"
            )));
        }

        let table_rows = sqlx::query(
            r#"
            SELECT table_name AS name,
                   table_comment AS comment
            FROM information_schema.tables
            WHERE table_schema = ?
              AND table_type = 'BASE TABLE'
            ORDER BY table_name
            "#,
        )
        .bind(&schema)
        .fetch_all(&pool)
        .await
        .map_err(|error| Error::Database(format!("reading MySQL/MariaDB tables failed: {error}")))?;

        let mut tables: BTreeMap<String, TableModel> = BTreeMap::new();
        for row in table_rows {
            let name: String = row.try_get("name").map_err(decode("table name"))?;
            let comment: String = row.try_get("comment").map_err(decode("table comment"))?;
            tables.insert(
                name.clone(),
                TableModel {
                    name,
                    comment: non_blank(Some(comment)),
                    columns: Vec::new(),
                },
            );
        }

        let column_rows = sqlx::query(
            r#"
            SELECT c.table_name AS table_name,
                   c.ordinal_position AS ordinal_position,
                   c.column_name AS name,
                   LOWER(c.data_type) AS type_name,
                   c.column_type AS formatted_type,
                   c.is_nullable AS is_nullable,
                   CAST(c.numeric_precision AS SIGNED) AS numeric_precision,
                   CAST(c.numeric_scale AS SIGNED) AS numeric_scale,
                   c.column_comment AS comment,
                   EXISTS (
                       SELECT 1
                       FROM information_schema.table_constraints tc
                       JOIN information_schema.key_column_usage kcu
                         ON kcu.constraint_schema = tc.constraint_schema
                        AND kcu.table_name = tc.table_name
                        AND kcu.constraint_name = tc.constraint_name
                       WHERE tc.constraint_schema = c.table_schema
                         AND tc.table_name = c.table_name
                         AND tc.constraint_type = 'PRIMARY KEY'
                         AND kcu.column_name = c.column_name
                   ) AS primary_key,
                   EXISTS (
                       SELECT 1
                       FROM information_schema.key_column_usage fk
                       WHERE fk.constraint_schema = c.table_schema
                         AND fk.table_name = c.table_name
                         AND fk.column_name = c.column_name
                         AND fk.referenced_table_name IS NOT NULL
                   ) AS foreign_key
            FROM information_schema.columns c
            WHERE c.table_schema = ?
            ORDER BY c.table_name, c.ordinal_position
            "#,
        )
        .bind(&schema)
        .fetch_all(&pool)
        .await
        .map_err(|error| Error::Database(format!("reading MySQL/MariaDB columns failed: {error}")))?;

        let mut enums: BTreeMap<String, EnumModel> = BTreeMap::new();
        for row in column_rows {
            let table_name: String = row.try_get("table_name").map_err(decode("table name"))?;
            let table = tables.get_mut(&table_name).ok_or_else(|| {
                Error::Database(format!(
                    "MySQL/MariaDB column metadata referenced unknown table '{table_name}'"
                ))
            })?;
            let name: String = row.try_get("name").map_err(decode("column name"))?;
            let type_name: String = row.try_get("type_name").map_err(decode("column type"))?;
            let formatted_type: String =
                row.try_get("formatted_type").map_err(decode("formatted type"))?;
            let enum_key = if type_name == "enum" {
                let key = format!("{table_name}.{name}");
                let values = parse_enum_values(&formatted_type)?;
                enums.insert(
                    key.clone(),
                    EnumModel {
                        rule_name: key.clone(),
                        graphql_name: type_name_for_mysql_enum(&name),
                        values,
                        comment: None,
                    },
                );
                Some(key)
            } else {
                None
            };
            let nullable: String = row.try_get("is_nullable").map_err(decode("nullability"))?;
            let primary_key: i64 =
                row.try_get("primary_key").map_err(decode("primary key flag"))?;
            let foreign_key: i64 =
                row.try_get("foreign_key").map_err(decode("foreign key flag"))?;

            table.columns.push(ColumnModel {
                name,
                type_name,
                formatted_type,
                enum_key,
                element_type_name: None,
                element_enum_key: None,
                array_dimensions: 0,
                nullable: nullable.eq_ignore_ascii_case("YES"),
                primary_key: primary_key != 0,
                foreign_key: foreign_key != 0,
                comment: non_blank(Some(
                    row.try_get("comment").map_err(decode("column comment"))?,
                )),
                numeric_precision: row
                    .try_get("numeric_precision")
                    .map_err(decode("numeric precision"))?,
                numeric_scale: row
                    .try_get("numeric_scale")
                    .map_err(decode("numeric scale"))?,
            });
        }

        Ok(SchemaModel {
            database_kind: DatabaseKind::Mysql,
            schema_name: schema,
            enums: enums.into_values().collect(),
            tables: tables.into_values().collect(),
        })
    }
    .await;

    pool.close().await;
    result
}

fn type_name_for_mysql_enum(column: &str) -> String {
    type_name(column)
}

fn parse_enum_values(column_type: &str) -> Result<Vec<String>> {
    let source = column_type.trim();
    let inner = source
        .strip_prefix("enum(")
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| {
            Error::Database(format!(
                "MySQL/MariaDB enum metadata has unexpected declaration '{column_type}'"
            ))
        })?;

    let mut values = Vec::new();
    let mut chars = inner.chars().peekable();
    loop {
        while matches!(chars.peek(), Some(c) if c.is_whitespace() || *c == ',') {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        if chars.next() != Some('\'') {
            return Err(Error::Database(format!(
                "MySQL/MariaDB enum metadata has unexpected declaration '{column_type}'"
            )));
        }
        let mut value = String::new();
        let mut closed = false;
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(escaped) = chars.next() {
                    value.push(escaped);
                } else {
                    value.push(ch);
                }
                continue;
            }
            if ch == '\'' {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    value.push('\'');
                    continue;
                }
                closed = true;
                break;
            }
            value.push(ch);
        }
        if !closed {
            return Err(Error::Database(format!(
                "MySQL/MariaDB enum metadata has unterminated value in '{column_type}'"
            )));
        }
        values.push(value);
    }
    if values.is_empty() {
        return Err(Error::Database(format!(
            "MySQL/MariaDB enum metadata has no values in '{column_type}'"
        )));
    }
    Ok(values)
}

fn decode(label: &'static str) -> impl FnOnce(sqlx::Error) -> Error {
    move |error| Error::Database(format!("decoding MySQL/MariaDB {label} failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mysql_enum_column_type() {
        assert_eq!(
            parse_enum_values("enum('PENDING','APPROVED','can\\'t')").unwrap(),
            ["PENDING", "APPROVED", "can't"]
        );
        assert_eq!(
            parse_enum_values("enum('one','two''s')").unwrap(),
            ["one", "two's"]
        );
        assert!(parse_enum_values("set('a','b')").is_err());
    }

    #[test]
    fn mysql_enum_names_follow_seaorm_column_name() {
        assert_eq!(type_name_for_mysql_enum("status"), "Status");
    }
}
