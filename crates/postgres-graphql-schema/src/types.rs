use std::collections::{BTreeMap, HashMap};

use crate::ColumnModel;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MappedType {
    pub expression: String,
    pub enum_name: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct TypeMapper {
    custom: HashMap<String, String>,
}

impl TypeMapper {
    pub fn new(custom: &BTreeMap<String, String>) -> Self {
        Self {
            custom: custom
                .iter()
                .map(|(sql, graphql)| (normalize(sql), graphql.trim().to_owned()))
                .collect(),
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
        if column.type_kind == "e" {
            return enum_names.get(&column.type_name).map(|name| MappedType {
                expression: name.clone(),
                enum_name: Some(column.type_name.clone()),
            });
        }
        self.scalar(&column.type_name)
    }

    fn map_array(
        &self,
        column: &ColumnModel,
        enum_names: &HashMap<String, String>,
    ) -> Option<MappedType> {
        let element_name = column.element_type_name.as_ref()?;
        let element = if column.element_type_kind.as_deref() == Some("e") {
            enum_names.get(element_name).map(|name| MappedType {
                expression: name.clone(),
                enum_name: Some(element_name.clone()),
            })
        } else {
            self.scalar(element_name)
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

    fn scalar(&self, sql_type: &str) -> Option<MappedType> {
        let normalized = normalize(sql_type);
        let expression = self
            .custom
            .get(&normalized)
            .cloned()
            .or_else(|| default_type(&normalized).map(str::to_owned))?;
        Some(MappedType {
            expression,
            enum_name: None,
        })
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn default_type(sql_type: &str) -> Option<&'static str> {
    match sql_type {
        "uuid" => Some("UUID"),
        "varchar" | "character varying" | "bpchar" | "character" | "char" | "text" | "name"
        | "citext" | "inet" | "cidr" | "macaddr" | "macaddr8" => Some("String"),
        "int2" | "smallint" | "int4" | "integer" => Some("Int"),
        "int8" | "bigint" => Some("Long"),
        "bool" | "boolean" => Some("Boolean"),
        "float4" | "real" | "float8" | "double precision" => Some("Float"),
        // SeaORM 2.x generates rust_decimal::Decimal for PostgreSQL numeric/decimal
        // regardless of scale. Keep decimal semantics rather than narrowing scale=0.
        "numeric" | "decimal" => Some("BigDecimal"),
        "date" => Some("Date"),
        "timestamp" | "timestamp without time zone" => Some("LocalDateTime"),
        "timestamptz" | "timestamp with time zone" => Some("DateTime"),
        "time" | "time without time zone" => Some("LocalTime"),
        "timetz" | "time with time zone" => Some("Time"),
        "json" | "jsonb" => Some("JSON"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(sql: &str) -> ColumnModel {
        ColumnModel::scalar("value", sql, false)
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
        ] {
            assert_eq!(
                TypeMapper::new(&BTreeMap::new())
                    .map(&column(sql), &HashMap::new())
                    .unwrap()
                    .expression,
                graphql,
                "{sql}"
            );
        }
    }

    #[test]
    fn custom_type_mapping_overrides_default() {
        let custom = BTreeMap::from([("jsonb".to_owned(), "String".to_owned())]);
        assert_eq!(
            TypeMapper::new(&custom)
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
        array.element_type_kind = Some("b".to_owned());
        assert_eq!(
            TypeMapper::new(&BTreeMap::new())
                .map(&array, &HashMap::new())
                .unwrap()
                .expression,
            "[[String!]!]"
        );

        let mut enum_array = column("_status");
        enum_array.array_dimensions = 1;
        enum_array.element_type_name = Some("review_status".to_owned());
        enum_array.element_type_kind = Some("e".to_owned());
        let enums = HashMap::from([("review_status".to_owned(), "ReviewStatus".to_owned())]);
        let mapped = TypeMapper::new(&BTreeMap::new())
            .map(&enum_array, &enums)
            .unwrap();
        assert_eq!(mapped.expression, "[ReviewStatus!]");
        assert_eq!(mapped.enum_name.as_deref(), Some("review_status"));
    }

    #[test]
    fn unknown_type_does_not_fallback() {
        assert!(
            TypeMapper::new(&BTreeMap::new())
                .map(&column("ltreeish_custom"), &HashMap::new())
                .is_none()
        );
    }
}
