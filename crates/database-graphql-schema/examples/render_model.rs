use std::path::PathBuf;

use database_graphql_schema::{
    ColumnModel, ColumnsConfig, Config, DatabaseConfig, DatabaseKind, EnumsConfig, Generator,
    GeneratorConfig, SchemaModel, TableConfig, TableModel, TypesConfig,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        database: DatabaseConfig::default(),
        generator: GeneratorConfig {
            output: PathBuf::from("entities.graphqls"),
            descriptions: true,
            header: Some(String::new()),
        },
        tables: TableConfig {
            include: vec![".*".to_owned()],
            exclude: vec![],
        },
        columns: ColumnsConfig::default(),
        types: TypesConfig::default(),
        enums: EnumsConfig::default(),
    };

    let schema = SchemaModel {
        database_kind: DatabaseKind::Postgres,
        schema_name: "public".to_owned(),
        enums: vec![],
        tables: vec![TableModel {
            name: "work_session".to_owned(),
            comment: Some("Persisted work sessions.".to_owned()),
            columns: vec![
                ColumnModel::scalar("id", "uuid", false),
                ColumnModel::scalar("display_name", "text", true),
            ],
        }],
    };

    let sdl = Generator.render(&config, &schema)?;
    assert!(sdl.contains("type WorkSession"));
    assert!(sdl.contains("id: UUID!"));
    assert!(sdl.contains("displayName: String"));
    print!("{sdl}");
    Ok(())
}
