use std::collections::{BTreeMap, HashMap};

use crate::{ColumnModel, DatabaseKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MappedType {
    pub expression: String,
    pub enum_name: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct TypeMapper {
    custom: HashMap<String, String>,
    database_kind: DatabaseKind,
}

impl TypeMapper {
    pub fn new(custom: &BTreeMap<String, String>, database_kind: DatabaseKind) -> Self {
        Self {
            custom: custom
                .iter()
                .map(|(sql, graphql)| (normalize(sql), graphql.trim().to_owned()))
                .collect(),
            database_kind,
        }
    }

    pub fn map(
        &self,
        column: &ColumnModel,
        enum_names: &HashMap<String, String>,
    ) -> Option<MappedType> {
        if column.array_dimensions > 0 {
            return self.map_array(column, enum_names);
        }
        if let Some(enum_key) = &column.enum_key {
            return enum_names.get(enum_key).map(|name| MappedType {
                expression: name.clone(),
                enum_name: Some(enum_key.clone()),
            });
        }
        self.scalar(column)
    }

    fn map_array(
        &self,
        column: &ColumnModel,
        enum_names: &HashMap<String, String>,
    ) -> Option<MappedType> {
        let element_name = column.element_type_name.as_ref()?;
        let element = if let Some(enum_key) = &column.element_enum_key {
            enum_names.get(enum_key).map(|name| MappedType {
                expression: name.clone(),
                enum_name: Some(enum_key.clone()),
            })
        } else {
            let mut element_column = ColumnModel::scalar(&column.name, element_name, false);
            element_column.formatted_type = element_name.clone();
            self.scalar(&element_column)
        }?;

        let dimensions = column.array_dimensions.max(1);
        let mut expression = element.expression;
        for _ in 0..dimensions {
            expression = format!("[{expression}!]");
        }
        Some(MappedType {
            expression,
            enum_name: element.enum_name,
        })
    }

    fn scalar(&self, column: &ColumnModel) -> Option<MappedType> {
        let normalized = normalize(&column.type_name);
        let expression = self
            .custom
            .get(&normalized)
            .cloned()
            .or_else(|| default_type(self.database_kind, column).map(str::to_owned))?;
        Some(MappedType {
            expression,
            enum_name: None,
        })
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn default_type(kind: DatabaseKind, column: &ColumnModel) -> Option<&'static str> {
    match kind {
        DatabaseKind::Postgres => postgres_type(&normalize(&column.type_name)),
        DatabaseKind::Mysql => mysql_type(column),
        DatabaseKind::Sqlite => sqlite_type(&normalize(&column.type_name)),
    }
}

fn postgres_type(sql_type: &str) -> Option<&'static str> {
    match sql_type {
        "uuid" => Some("UUID"),
        "varchar" | "character varying" | "bpchar" | "character" | "char" | "text" | "name"
        | "citext" | "inet" | "cidr" | "macaddr" | "macaddr8" => Some("String"),
        "int2" | "smallint" | "int4" | "integer" => Some("Int"),
        "int8" | "bigint" => Some("Long"),
        "bool" | "boolean" => Some("Boolean"),
        "float4" | "real" | "float8" | "double precision" => Some("Float"),
        "numeric" | "decimal" => Some("BigDecimal"),
        "date" => Some("Date"),
        "timestamp" | "timestamp without time zone" => Some("LocalDateTime"),
        "timestamptz" | "timestamp with time zone" => Some("DateTime"),
        "time" | "time without time zone" => Some("LocalTime"),
        "timetz" | "time with time zone" => Some("Time"),
        "json" | "jsonb" => Some("JSON"),
        "bytea" => Some("Bytes"),
        _ => None,
    }
}

fn mysql_type(column: &ColumnModel) -> Option<&'static str> {
    let sql_type = normalize(&column.type_name);
    match sql_type.as_str() {
        "char" | "varchar" | "tinytext" | "text" | "mediumtext" | "longtext" => Some("String"),
        // MySQL BOOLEAN is an alias for TINYINT(1), and information_schema does not
        // preserve whether the DDL used BOOLEAN. SeaORM 2.0.x therefore emits i8.
        // Consumers that own semantic boolean intent can explicitly override it.
        "tinyint" | "smallint" | "mediumint" | "int" | "integer" => Some("Int"),
        "bigint" => Some("Long"),
        "bool" | "boolean" => Some("Boolean"),
        "float" | "double" => Some("Float"),
        "decimal" | "numeric" => Some("BigDecimal"),
        "date" => Some("Date"),
        "datetime" => Some("LocalDateTime"),
        "timestamp" => Some("DateTime"),
        "time" => Some("LocalTime"),
        "year" => Some("Int"),
        "json" => Some("JSON"),
        "binary" | "varbinary" | "tinyblob" | "blob" | "mediumblob" | "longblob" | "bit" => {
            Some("Bytes")
        }
        _ => None,
    }
}

fn sqlite_type(sql_type: &str) -> Option<&'static str> {
    match sql_type {
        "uuid" => Some("UUID"),
        "char" | "varchar" | "text" => Some("String"),
        "tinyint" | "smallint" => Some("Int"),
        // SQLite INTEGER is signed 64-bit; SeaORM maps INTEGER/BIGINT to i64.
        "integer" => Some("Long"),
        "boolean" => Some("Boolean"),
        "float" | "double" | "real" => Some("Float"),
        "decimal" => Some("BigDecimal"),
        "date_text" => Some("Date"),
        "datetime_text" => Some("LocalDateTime"),
        "timestamp" | "timestamp_with_timezone_text" => Some("DateTime"),
        "time_text" => Some("LocalTime"),
        "json" => Some("JSON"),
        "blob" => Some("Bytes"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(sql: &str) -> ColumnModel {
        ColumnModel::scalar("value", sql, false)
    }

    fn map(kind: DatabaseKind, sql: &str) -> Option<String> {
        TypeMapper::new(&BTreeMap::new(), kind)
            .map(&column(sql), &HashMap::new())
            .map(|value| value.expression)
    }

    #[test]
    fn maps_postgres_baseline_types() {
        for (sql, graphql) in [
            ("uuid", "UUID"),
            ("varchar", "String"),
            ("bpchar", "String"),
            ("text", "String"),
            ("inet", "String"),
            ("int2", "Int"),
            ("int4", "Int"),
            ("int8", "Long"),
            ("bool", "Boolean"),
            ("float4", "Float"),
            ("float8", "Float"),
            ("numeric", "BigDecimal"),
            ("date", "Date"),
            ("timestamp", "LocalDateTime"),
            ("timestamptz", "DateTime"),
            ("time", "LocalTime"),
            ("timetz", "Time"),
            ("json", "JSON"),
            ("jsonb", "JSON"),
            ("bytea", "Bytes"),
        ] {
            assert_eq!(
                map(DatabaseKind::Postgres, sql).as_deref(),
                Some(graphql),
                "{sql}"
            );
        }
    }

    #[test]
    fn maps_mysql_baseline_types() {
        for (sql, graphql) in [
            ("varchar", "String"),
            ("int", "Int"),
            ("bigint", "Long"),
            ("decimal", "BigDecimal"),
            ("datetime", "LocalDateTime"),
            ("timestamp", "DateTime"),
            ("json", "JSON"),
            ("blob", "Bytes"),
        ] {
            assert_eq!(
                map(DatabaseKind::Mysql, sql).as_deref(),
                Some(graphql),
                "{sql}"
            );
        }
        let mut boolean_alias = column("tinyint");
        boolean_alias.formatted_type = "tinyint(1)".to_owned();
        assert_eq!(
            TypeMapper::new(&BTreeMap::new(), DatabaseKind::Mysql)
                .map(&boolean_alias, &HashMap::new())
                .unwrap()
                .expression,
            "Int"
        );
    }

    #[test]
    fn maps_sqlite_baseline_types() {
        for (sql, graphql) in [
            ("integer", "Long"),
            ("text", "String"),
            ("real", "Float"),
            ("decimal", "BigDecimal"),
            ("boolean", "Boolean"),
            ("uuid", "UUID"),
            ("json", "JSON"),
            ("blob", "Bytes"),
        ] {
            assert_eq!(
                map(DatabaseKind::Sqlite, sql).as_deref(),
                Some(graphql),
                "{sql}"
            );
        }
    }

    #[test]
    fn custom_type_mapping_overrides_default() {
        let custom = BTreeMap::from([("jsonb".to_owned(), "String".to_owned())]);
        assert_eq!(
            TypeMapper::new(&custom, DatabaseKind::Postgres)
                .map(&column("jsonb"), &HashMap::new())
                .unwrap()
                .expression,
            "String"
        );
    }

    #[test]
    fn maps_multidimensional_arrays_and_enum_arrays() {
        let mut array = column("_text");
        array.array_dimensions = 2;
        array.element_type_name = Some("text".to_owned());
        assert_eq!(
            TypeMapper::new(&BTreeMap::new(), DatabaseKind::Postgres)
                .map(&array, &HashMap::new())
                .unwrap()
                .expression,
            "[[String!]!]"
        );

        let mut enum_array = column("_status");
        enum_array.array_dimensions = 1;
        enum_array.element_type_name = Some("review_status".to_owned());
        enum_array.element_enum_key = Some("review_status".to_owned());
        let enums = HashMap::from([("review_status".to_owned(), "ReviewStatus".to_owned())]);
        let mapped = TypeMapper::new(&BTreeMap::new(), DatabaseKind::Postgres)
            .map(&enum_array, &enums)
            .unwrap();
        assert_eq!(mapped.expression, "[ReviewStatus!]");
        assert_eq!(mapped.enum_name.as_deref(), Some("review_status"));
    }

    #[test]
    fn unknown_type_does_not_fallback() {
        assert!(
            TypeMapper::new(&BTreeMap::new(), DatabaseKind::Postgres)
                .map(&column("ltreeish_custom"), &HashMap::new())
                .is_none()
        );
    }
}
