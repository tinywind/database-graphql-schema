use std::{collections::BTreeMap, time::Duration};

use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use crate::{
    ColumnModel, EnumModel, Error, Result, SchemaModel, TableModel, error::database_error,
};

#[derive(Clone, Debug, Default)]
pub struct PostgresMetadataReader;

impl PostgresMetadataReader {
    pub async fn read(&self, database_url: &str, schema: &str) -> Result<SchemaModel> {
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await
            .map_err(|error| database_error(database_url, "connection failed", error))?;
        let result = self.read_pool(&pool, schema).await;
        pool.close().await;
        result
    }

    pub async fn read_pool(&self, pool: &PgPool, schema: &str) -> Result<SchemaModel> {
        let schema_exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
                .bind(schema)
                .fetch_one(pool)
                .await
                .map_err(|error| {
                    Error::Database(format!("checking schema visibility failed: {error}"))
                })?;
        if !schema_exists {
            return Err(Error::Database(format!(
                "schema '{schema}' is not visible to the database connection"
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
        .fetch_all(pool)
        .await
        .map_err(|error| Error::Database(format!("reading tables failed: {error}")))?;

        let mut tables: BTreeMap<String, TableModel> = BTreeMap::new();
        for row in table_rows {
            let name: String = row
                .try_get("name")
                .map_err(|error| Error::Database(format!("decoding table name failed: {error}")))?;
            let comment: Option<String> = row.try_get("comment").map_err(|error| {
                Error::Database(format!("decoding table comment failed: {error}"))
            })?;
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
                   tn.nspname::text AS type_schema,
                   t.typtype::text AS type_kind,
                   format_type(a.atttypid, a.atttypmod)::text AS formatted_type,
                   a.attnotnull AS not_null,
                   a.attndims::int4 AS array_dimensions,
                   et.typname::text AS element_type_name,
                   en.nspname::text AS element_type_schema,
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
            JOIN pg_namespace tn ON tn.oid = t.typnamespace
            LEFT JOIN pg_type et ON et.oid = t.typelem AND t.typelem <> 0
            LEFT JOIN pg_namespace en ON en.oid = et.typnamespace
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
        .fetch_all(pool)
        .await
        .map_err(|error| Error::Database(format!("reading columns failed: {error}")))?;

        for row in column_rows {
            let table_name: String = row.try_get("table_name").map_err(decode("table name"))?;
            let table = tables.get_mut(&table_name).ok_or_else(|| {
                Error::Database(format!(
                    "column metadata referenced unknown table '{table_name}'"
                ))
            })?;
            let array_dimensions: i32 = row
                .try_get("array_dimensions")
                .map_err(decode("array dimensions"))?;
            table.columns.push(ColumnModel {
                name: row.try_get("name").map_err(decode("column name"))?,
                type_name: row.try_get("type_name").map_err(decode("column type"))?,
                type_schema: row.try_get("type_schema").map_err(decode("type schema"))?,
                type_kind: row.try_get("type_kind").map_err(decode("type kind"))?,
                formatted_type: row
                    .try_get("formatted_type")
                    .map_err(decode("formatted type"))?,
                element_type_name: row
                    .try_get("element_type_name")
                    .map_err(decode("array element type"))?,
                element_type_schema: row
                    .try_get("element_type_schema")
                    .map_err(decode("array element schema"))?,
                element_type_kind: row
                    .try_get("element_type_kind")
                    .map_err(decode("array element kind"))?,
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
        .fetch_all(pool)
        .await
        .map_err(|error| Error::Database(format!("reading enums failed: {error}")))?;

        let mut enums: BTreeMap<String, EnumModel> = BTreeMap::new();
        for row in enum_rows {
            let name: String = row.try_get("name").map_err(decode("enum name"))?;
            let comment: Option<String> = row.try_get("comment").map_err(decode("enum comment"))?;
            let value: String = row.try_get("value").map_err(decode("enum value"))?;
            enums
                .entry(name.clone())
                .or_insert_with(|| EnumModel {
                    name,
                    values: Vec::new(),
                    comment: non_blank(comment),
                })
                .values
                .push(value);
        }

        Ok(SchemaModel {
            schema_name: schema.to_owned(),
            enums: enums.into_values().collect(),
            tables: tables.into_values().collect(),
        })
    }
}

fn non_blank(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

fn decode(label: &'static str) -> impl FnOnce(sqlx::Error) -> Error {
    move |error| Error::Database(format!("decoding {label} failed: {error}"))
}
