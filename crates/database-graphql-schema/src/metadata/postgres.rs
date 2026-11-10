use std::{collections::BTreeMap, time::Duration};

use sqlx::{Row, postgres::PgPoolOptions};

use crate::{
    ColumnModel, DatabaseKind, EnumModel, Error, Result, SchemaModel, TableModel,
    error::database_error, naming::type_name,
};

use super::non_blank;

pub(super) async fn read(database_url: &str, schema: &str) -> Result<SchemaModel> {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
        .map_err(|error| database_error(database_url, "PostgreSQL connection failed", error))?;

    let result = async {
        let schema_exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
                .bind(schema)
                .fetch_one(&pool)
                .await
                .map_err(|error| {
                    Error::Database(format!(
                        "checking PostgreSQL schema visibility failed: {error}"
                    ))
                })?;
        if !schema_exists {
            return Err(Error::Database(format!(
                "PostgreSQL schema '{schema}' is not visible to the database connection"
            )));
        }

        let table_rows = sqlx::query(
            r#"
            SELECT c.relname::text AS name,
                   obj_description(c.oid, 'pg_class') AS comment
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = $1
              AND c.relkind IN ('r', 'p')
            ORDER BY c.relname
            "#,
        )
        .bind(schema)
        .fetch_all(&pool)
        .await
        .map_err(|error| Error::Database(format!("reading PostgreSQL tables failed: {error}")))?;

        let mut tables: BTreeMap<String, TableModel> = BTreeMap::new();
        for row in table_rows {
            let name: String = row.try_get("name").map_err(decode("table name"))?;
            let comment: Option<String> =
                row.try_get("comment").map_err(decode("table comment"))?;
            tables.insert(
                name.clone(),
                TableModel {
                    name,
                    comment: non_blank(comment),
                    columns: Vec::new(),
                },
            );
        }

        let column_rows = sqlx::query(
            r#"
            SELECT c.relname::text AS table_name,
                   a.attnum::int4 AS column_order,
                   a.attname::text AS name,
                   t.typname::text AS type_name,
                   t.typtype::text AS type_kind,
                   format_type(a.atttypid, a.atttypmod)::text AS formatted_type,
                   a.attnotnull AS not_null,
                   a.attndims::int4 AS array_dimensions,
                   et.typname::text AS element_type_name,
                   et.typtype::text AS element_type_kind,
                   col_description(c.oid, a.attnum) AS comment,
                   ic.numeric_precision::int4 AS numeric_precision,
                   ic.numeric_scale::int4 AS numeric_scale,
                   EXISTS (
                       SELECT 1
                       FROM pg_constraint con
                       WHERE con.conrelid = c.oid
                         AND con.contype = 'p'
                         AND a.attnum = ANY(con.conkey)
                   ) AS primary_key,
                   EXISTS (
                       SELECT 1
                       FROM pg_constraint con
                       WHERE con.conrelid = c.oid
                         AND con.contype = 'f'
                         AND a.attnum = ANY(con.conkey)
                   ) AS foreign_key
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            JOIN pg_attribute a ON a.attrelid = c.oid
            JOIN pg_type t ON t.oid = a.atttypid
            LEFT JOIN pg_type et ON et.oid = t.typelem AND t.typelem <> 0
            LEFT JOIN information_schema.columns ic
              ON ic.table_schema = n.nspname
             AND ic.table_name = c.relname
             AND ic.column_name = a.attname
            WHERE n.nspname = $1
              AND c.relkind IN ('r', 'p')
              AND a.attnum > 0
              AND NOT a.attisdropped
            ORDER BY c.relname, a.attnum
            "#,
        )
        .bind(schema)
        .fetch_all(&pool)
        .await
        .map_err(|error| Error::Database(format!("reading PostgreSQL columns failed: {error}")))?;

        for row in column_rows {
            let table_name: String = row.try_get("table_name").map_err(decode("table name"))?;
            let table = tables.get_mut(&table_name).ok_or_else(|| {
                Error::Database(format!(
                    "PostgreSQL column metadata referenced unknown table '{table_name}'"
                ))
            })?;
            let type_name: String = row.try_get("type_name").map_err(decode("column type"))?;
            let type_kind: String = row.try_get("type_kind").map_err(decode("type kind"))?;
            let element_type_name: Option<String> = row
                .try_get("element_type_name")
                .map_err(decode("array element type"))?;
            let element_type_kind: Option<String> = row
                .try_get("element_type_kind")
                .map_err(decode("array element kind"))?;
            let array_dimensions: i32 = row
                .try_get("array_dimensions")
                .map_err(decode("array dimensions"))?;

            table.columns.push(ColumnModel {
                name: row.try_get("name").map_err(decode("column name"))?,
                enum_key: (type_kind == "e").then(|| type_name.clone()),
                element_enum_key: (element_type_kind.as_deref() == Some("e"))
                    .then(|| element_type_name.clone())
                    .flatten(),
                type_name,
                formatted_type: row
                    .try_get("formatted_type")
                    .map_err(decode("formatted type"))?,
                element_type_name,
                array_dimensions: usize::try_from(array_dimensions.max(0)).unwrap_or(0),
                nullable: !row
                    .try_get::<bool, _>("not_null")
                    .map_err(decode("nullability"))?,
                primary_key: row
                    .try_get("primary_key")
                    .map_err(decode("primary key flag"))?,
                foreign_key: row
                    .try_get("foreign_key")
                    .map_err(decode("foreign key flag"))?,
                comment: non_blank(row.try_get("comment").map_err(decode("column comment"))?),
                numeric_precision: row
                    .try_get("numeric_precision")
                    .map_err(decode("numeric precision"))?,
                numeric_scale: row
                    .try_get("numeric_scale")
                    .map_err(decode("numeric scale"))?,
            });
        }

        let enum_rows = sqlx::query(
            r#"
            SELECT t.typname::text AS name,
                   obj_description(t.oid, 'pg_type') AS comment,
                   e.enumlabel::text AS value
            FROM pg_type t
            JOIN pg_namespace n ON n.oid = t.typnamespace
            JOIN pg_enum e ON e.enumtypid = t.oid
            WHERE n.nspname = $1
              AND t.typtype = 'e'
            ORDER BY t.typname, e.enumsortorder
            "#,
        )
        .bind(schema)
        .fetch_all(&pool)
        .await
        .map_err(|error| Error::Database(format!("reading PostgreSQL enums failed: {error}")))?;

        let mut enums: BTreeMap<String, EnumModel> = BTreeMap::new();
        for row in enum_rows {
            let name: String = row.try_get("name").map_err(decode("enum name"))?;
            let comment: Option<String> = row.try_get("comment").map_err(decode("enum comment"))?;
            let value: String = row.try_get("value").map_err(decode("enum value"))?;
            enums
                .entry(name.clone())
                .or_insert_with(|| EnumModel {
                    rule_name: name.clone(),
                    graphql_name: type_name(&name),
                    values: Vec::new(),
                    comment: non_blank(comment),
                })
                .values
                .push(value);
        }

        Ok(SchemaModel {
            database_kind: DatabaseKind::Postgres,
            schema_name: schema.to_owned(),
            enums: enums.into_values().collect(),
            tables: tables.into_values().collect(),
        })
    }
    .await;

    pool.close().await;
    result
}

fn decode(label: &'static str) -> impl FnOnce(sqlx::Error) -> Error {
    move |error| Error::Database(format!("decoding PostgreSQL {label} failed: {error}"))
}
