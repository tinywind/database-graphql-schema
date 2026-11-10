use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseKind {
    Postgres,
    Mysql,
    Sqlite,
}

impl fmt::Display for DatabaseKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Postgres => "PostgreSQL",
            Self::Mysql => "MySQL/MariaDB",
            Self::Sqlite => "SQLite",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaModel {
    pub database_kind: DatabaseKind,
    pub schema_name: String,
    pub enums: Vec<EnumModel>,
    pub tables: Vec<TableModel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumModel {
    /// Name used by config matching and column references.
    pub rule_name: String,
    /// Already-resolved GraphQL type name. MySQL ENUM has no schema-level SQL type,
    /// so it uses a deterministic table+column name here.
    pub graphql_name: String,
    pub values: Vec<String>,
    pub comment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableModel {
    pub name: String,
    pub comment: Option<String>,
    pub columns: Vec<ColumnModel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnModel {
    pub name: String,
    /// Normalized DBMS type name (for example uuid, varchar, enum, integer).
    pub type_name: String,
    /// DBMS-formatted declaration used in diagnostics.
    pub formatted_type: String,
    /// Config/enum key when this column is a native enum.
    pub enum_key: Option<String>,
    pub element_type_name: Option<String>,
    pub element_enum_key: Option<String>,
    pub array_dimensions: usize,
    pub nullable: bool,
    pub primary_key: bool,
    pub foreign_key: bool,
    pub comment: Option<String>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
}

impl ColumnModel {
    pub fn scalar(name: &str, type_name: &str, nullable: bool) -> Self {
        Self {
            name: name.to_owned(),
            type_name: type_name.to_owned(),
            formatted_type: type_name.to_owned(),
            enum_key: None,
            element_type_name: None,
            element_enum_key: None,
            array_dimensions: 0,
            nullable,
            primary_key: false,
            foreign_key: false,
            comment: None,
            numeric_precision: None,
            numeric_scale: None,
        }
    }
}
