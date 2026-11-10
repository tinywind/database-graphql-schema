#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaModel {
    pub schema_name: String,
    pub enums: Vec<EnumModel>,
    pub tables: Vec<TableModel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumModel {
    pub name: String,
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
    pub type_name: String,
    pub type_schema: String,
    pub type_kind: String,
    pub formatted_type: String,
    pub element_type_name: Option<String>,
    pub element_type_schema: Option<String>,
    pub element_type_kind: Option<String>,
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
            type_schema: "pg_catalog".to_owned(),
            type_kind: "b".to_owned(),
            formatted_type: type_name.to_owned(),
            element_type_name: None,
            element_type_schema: None,
            element_type_kind: None,
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
