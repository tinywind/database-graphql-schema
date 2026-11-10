use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    ColumnModel, Config, Error, Result, SchemaModel,
    metadata::DatabaseMetadataReader,
    naming::{field_name, type_name, validate_enum_value, validate_field_name, validate_type_name},
    render::{SdlDocument, SdlEnum, SdlField, SdlType, render},
    rules::{OverrideRules, Rules},
    types::TypeMapper,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedSchema {
    pub file: PathBuf,
    pub type_count: usize,
    pub enum_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verification {
    Match {
        file: PathBuf,
    },
    Missing {
        file: PathBuf,
    },
    Different {
        file: PathBuf,
        line: usize,
        expected: Option<String>,
        actual: Option<String>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct Generator;

impl Generator {
    pub fn render(&self, config: &Config, schema: &SchemaModel) -> Result<String> {
        let document = self.build_document(config, schema)?;
        Ok(render(&document))
    }

    pub fn generate(&self, config: &Config, schema: &SchemaModel) -> Result<GeneratedSchema> {
        let document = self.build_document(config, schema)?;
        let rendered = render(&document);
        write_output(&config.generator.output, &rendered)?;
        Ok(GeneratedSchema {
            file: config.generator.output.clone(),
            type_count: document.types.len(),
            enum_count: document.enums.len(),
        })
    }

    pub fn verify(&self, config: &Config, schema: &SchemaModel) -> Result<Verification> {
        let expected = self.render(config, schema)?;
        let file = &config.generator.output;
        if !file.is_file() {
            return Ok(Verification::Missing { file: file.clone() });
        }
        let actual = fs::read_to_string(file).map_err(|error| Error::OutputRead {
            path: file.clone(),
            message: error.to_string(),
        })?;
        if actual == expected {
            return Ok(Verification::Match { file: file.clone() });
        }

        let expected_lines: Vec<_> = expected.lines().collect();
        let actual_lines: Vec<_> = actual.lines().collect();
        let length = expected_lines.len().max(actual_lines.len());
        let index = (0..length)
            .find(|index| expected_lines.get(*index) != actual_lines.get(*index))
            .unwrap_or(0);
        Ok(Verification::Different {
            file: file.clone(),
            line: index + 1,
            expected: expected_lines.get(index).map(|line| (*line).to_owned()),
            actual: actual_lines.get(index).map(|line| (*line).to_owned()),
        })
    }

    fn build_document(&self, config: &Config, schema: &SchemaModel) -> Result<SdlDocument> {
        let selected_tables = select_tables(config, schema)?;
        let enum_type_names: HashMap<String, String> = schema
            .enums
            .iter()
            .map(|value| (value.rule_name.clone(), value.graphql_name.clone()))
            .collect();

        let mut column_rules = ColumnRules::new(config)?;
        let mapper = TypeMapper::new(&config.types.sql, schema.database_kind);
        let mut referenced_enums = HashSet::new();
        let mut types = Vec::with_capacity(selected_tables.len());

        for table in selected_tables {
            let name = type_name(&table.name);
            validate_type_name(&name, &format!("Table {}", table.name))?;
            let mut fields = Vec::new();
            for column in &table.columns {
                let qualified = format!("{}.{}", table.name, column.name);
                let override_type = column_rules.type_override(&qualified);
                let non_null = column_rules.non_null(&qualified, column)?;
                if !column_rules.keeps(&qualified, column) {
                    continue;
                }

                let mapped = if let Some(graphql) = override_type {
                    (graphql, None)
                } else {
                    let mapped = mapper.map(column, &enum_type_names).ok_or_else(|| {
                        Error::Generation(format!(
                            "Column {qualified} has the {} SQL type '{}' with no GraphQL mapping; add [types.sql], [[types.override]], or columns.exclude",
                            schema.database_kind, column.formatted_type
                        ))
                    })?;
                    (mapped.expression, mapped.enum_name)
                };
                if let Some(enum_name) = mapped.1 {
                    referenced_enums.insert(enum_name);
                }
                let name = field_name(&column.name);
                validate_field_name(&name, &format!("Column {qualified}"))?;
                fields.push(SdlField {
                    name,
                    graphql_type: mapped.0,
                    non_null,
                    description: column
                        .comment
                        .clone()
                        .filter(|_| config.generator.descriptions),
                });
            }

            if fields.is_empty() {
                return Err(Error::Generation(format!(
                    "Table {} keeps no column after the column rules; include a column or exclude the table",
                    table.name
                )));
            }
            let mut seen = HashSet::new();
            for field in &fields {
                if !seen.insert(field.name.clone()) {
                    return Err(Error::Generation(format!(
                        "Table {} has two columns that both map to the field '{}'; exclude one of them",
                        table.name, field.name
                    )));
                }
            }
            types.push(SdlType {
                name,
                fields,
                description: table
                    .comment
                    .clone()
                    .filter(|_| config.generator.descriptions),
            });
        }
        column_rules.assert_matched()?;

        let mut enum_include = Rules::new("enums.include", &config.enums.include)?;
        let mut enums: Vec<SdlEnum> = Vec::new();
        for value in &schema.enums {
            let forced = enum_include.any_matches(&value.rule_name);
            if !(config.enums.include_unreferenced
                || forced
                || referenced_enums.contains(&value.rule_name))
            {
                continue;
            }
            validate_type_name(&value.graphql_name, &format!("Enum {}", value.rule_name))?;
            if value.values.is_empty() {
                return Err(Error::Generation(format!(
                    "Enum {} has no value",
                    value.rule_name
                )));
            }
            for literal in &value.values {
                validate_enum_value(literal, &format!("Enum {}", value.rule_name))?;
            }
            let candidate = SdlEnum {
                name: value.graphql_name.clone(),
                values: value.values.clone(),
                description: value
                    .comment
                    .clone()
                    .filter(|_| config.generator.descriptions),
            };
            if let Some(existing) = enums.iter().find(|item| item.name == candidate.name) {
                if existing.values != candidate.values
                    || existing.description != candidate.description
                {
                    return Err(Error::Generation(format!(
                        "The GraphQL enum name '{}' is produced by incompatible database enums; rename or override/exclude one source",
                        candidate.name
                    )));
                }
            } else {
                enums.push(candidate);
            }
        }
        enum_include.assert_matched()?;

        enums.sort_by(|a, b| a.name.cmp(&b.name));
        types.sort_by(|a, b| a.name.cmp(&b.name));
        assert_unique_type_names(&enums, &types)?;

        let header = match &config.generator.header {
            Some(header) if header.is_empty() => None,
            Some(header) => Some(header.clone()),
            None => Some(format!(
                "# Generated by database-graphql-schema from {} schema \"{}\". Do not edit.",
                schema.database_kind, schema.schema_name
            )),
        };

        Ok(SdlDocument {
            header,
            enums,
            types,
        })
    }
}

pub async fn render_from_database(config: &Config, database_url: &str) -> Result<String> {
    let schema = DatabaseMetadataReader
        .read(database_url, &config.database)
        .await?;
    Generator.render(config, &schema)
}

pub async fn generate_from_database(
    config: &Config,
    database_url: &str,
) -> Result<GeneratedSchema> {
    let schema = DatabaseMetadataReader
        .read(database_url, &config.database)
        .await?;
    Generator.generate(config, &schema)
}

pub async fn verify_from_database(config: &Config, database_url: &str) -> Result<Verification> {
    let schema = DatabaseMetadataReader
        .read(database_url, &config.database)
        .await?;
    Generator.verify(config, &schema)
}

fn select_tables<'a>(
    config: &Config,
    schema: &'a SchemaModel,
) -> Result<Vec<&'a crate::TableModel>> {
    if config.tables.include.is_empty() {
        return Err(Error::Generation(
            "tables.include names no table; list tables to expose, or include [\".*\"] for every table"
                .to_owned(),
        ));
    }
    let mut includes = Rules::new("tables.include", &config.tables.include)?;
    let mut excludes = Rules::new("tables.exclude", &config.tables.exclude)?;
    let selected: Vec<_> = schema
        .tables
        .iter()
        .filter(|table| {
            let included = includes.any_matches(&table.name);
            let excluded = excludes.any_matches(&table.name);
            included && !excluded
        })
        .collect();
    includes.assert_matched()?;
    excludes.assert_matched()?;
    if selected.is_empty() {
        return Err(Error::Generation(format!(
            "tables rules select no table of {} schema '{}'",
            schema.database_kind, schema.schema_name
        )));
    }
    Ok(selected)
}

struct ColumnRules {
    includes: Rules,
    excludes: Rules,
    overrides: OverrideRules,
    nullable: Rules,
    non_null: Rules,
    exclude_foreign_keys: bool,
    keep_primary_keys: bool,
}

impl ColumnRules {
    fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            includes: Rules::new("columns.include", &config.columns.include)?,
            excludes: Rules::new("columns.exclude", &config.columns.exclude)?,
            overrides: OverrideRules::new(&config.types.overrides)?,
            nullable: Rules::new("types.nullable", &config.types.nullable)?,
            non_null: Rules::new("types.non_null", &config.types.non_null)?,
            exclude_foreign_keys: config.columns.exclude_foreign_keys,
            keep_primary_keys: config.columns.keep_primary_keys,
        })
    }

    fn keeps(&mut self, qualified: &str, column: &ColumnModel) -> bool {
        let excluded = self.excludes.any_matches(qualified);
        let included = self.includes.any_matches(qualified);
        if excluded {
            false
        } else if included {
            true
        } else if column.foreign_key && self.exclude_foreign_keys {
            column.primary_key && self.keep_primary_keys
        } else {
            true
        }
    }

    fn type_override(&mut self, qualified: &str) -> Option<String> {
        self.overrides.first_match(qualified)
    }

    fn non_null(&mut self, qualified: &str, column: &ColumnModel) -> Result<bool> {
        let nullable = self.nullable.any_matches(qualified);
        let non_null = self.non_null.any_matches(qualified);
        match (nullable, non_null) {
            (true, true) => Err(Error::Generation(format!(
                "Column {qualified} matches both types.nullable and types.non_null"
            ))),
            (true, false) => Ok(false),
            (false, true) => Ok(true),
            (false, false) => Ok(!column.nullable),
        }
    }

    fn assert_matched(&self) -> Result<()> {
        self.includes.assert_matched()?;
        self.excludes.assert_matched()?;
        self.overrides.assert_matched()?;
        self.nullable.assert_matched()?;
        self.non_null.assert_matched()
    }
}

fn assert_unique_type_names(enums: &[SdlEnum], types: &[SdlType]) -> Result<()> {
    let mut owners = HashMap::<&str, &str>::new();
    for value in enums {
        if owners.insert(&value.name, "enum").is_some() {
            return Err(Error::Generation(format!(
                "The GraphQL name '{}' is produced by more than one object",
                value.name
            )));
        }
    }
    for value in types {
        if let Some(previous) = owners.insert(&value.name, "table") {
            return Err(Error::Generation(format!(
                "The GraphQL name '{}' is produced by more than one object ({previous}, table)",
                value.name
            )));
        }
    }
    Ok(())
}

fn write_output(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::OutputWrite {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::write(path, content).map_err(|error| Error::OutputWrite {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::{
        ColumnsConfig, DatabaseConfig, DatabaseKind, EnumModel, EnumsConfig, GeneratorConfig,
        TableConfig, TableModel, TypeOverrideConfig, TypesConfig,
    };

    use super::*;

    fn base_config() -> Config {
        Config {
            database: DatabaseConfig::default(),
            generator: GeneratorConfig {
                output: PathBuf::from("out.graphqls"),
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
        }
    }

    fn schema() -> SchemaModel {
        let mut committee_id = ColumnModel::scalar("committee_id", "uuid", true);
        committee_id.foreign_key = true;
        let mut parent_id = ColumnModel::scalar("parent_id", "uuid", false);
        parent_id.foreign_key = true;
        parent_id.primary_key = true;
        let mut status = ColumnModel::scalar("status", "review_status", false);
        status.enum_key = Some("review_status".to_owned());

        SchemaModel {
            database_kind: DatabaseKind::Postgres,
            schema_name: "public".to_owned(),
            enums: vec![
                EnumModel {
                    rule_name: "review_status".to_owned(),
                    graphql_name: "ReviewStatus".to_owned(),
                    values: vec!["PENDING".to_owned(), "APPROVED".to_owned()],
                    comment: Some("Review state.".to_owned()),
                },
                EnumModel {
                    rule_name: "unused".to_owned(),
                    graphql_name: "Unused".to_owned(),
                    values: vec!["VALUE".to_owned()],
                    comment: None,
                },
            ],
            tables: vec![TableModel {
                name: "work_item".to_owned(),
                comment: Some("Work items.".to_owned()),
                columns: vec![
                    ColumnModel::scalar("id", "uuid", false),
                    committee_id,
                    parent_id,
                    status,
                    ColumnModel::scalar("display_name", "text", true),
                ],
            }],
        }
    }

    #[test]
    fn generates_types_enums_fk_rules_and_descriptions() {
        let output = Generator.render(&base_config(), &schema()).unwrap();
        assert!(output.starts_with("\"Review state.\"\nenum ReviewStatus"));
        assert!(output.contains("\"Work items.\"\ntype WorkItem"));
        assert!(output.contains("    id: UUID!"));
        assert!(!output.contains("committeeId"));
        assert!(output.contains("    parentId: UUID!"));
        assert!(output.contains("    status: ReviewStatus!"));
        assert!(output.contains("    displayName: String"));
        assert!(!output.contains("enum Unused"));
    }

    #[test]
    fn mysql_inline_enum_uses_seaorm_column_name() {
        let mut status = ColumnModel::scalar("status", "enum", false);
        status.enum_key = Some("account.status".to_owned());
        let schema = SchemaModel {
            database_kind: DatabaseKind::Mysql,
            schema_name: "app".to_owned(),
            enums: vec![EnumModel {
                rule_name: "account.status".to_owned(),
                graphql_name: "Status".to_owned(),
                values: vec!["PENDING".to_owned(), "ACTIVE".to_owned()],
                comment: None,
            }],
            tables: vec![TableModel {
                name: "account".to_owned(),
                comment: None,
                columns: vec![ColumnModel::scalar("id", "bigint", false), status],
            }],
        };
        let output = Generator.render(&base_config(), &schema).unwrap();
        assert!(output.contains("enum Status"));
        assert!(output.contains("status: Status!"));
    }

    #[test]
    fn incompatible_same_named_mysql_enums_fail_closed() {
        let mut first = ColumnModel::scalar("status", "enum", false);
        first.enum_key = Some("account.status".to_owned());
        let mut second = ColumnModel::scalar("status", "enum", false);
        second.enum_key = Some("project.status".to_owned());
        let schema = SchemaModel {
            database_kind: DatabaseKind::Mysql,
            schema_name: "app".to_owned(),
            enums: vec![
                EnumModel {
                    rule_name: "account.status".to_owned(),
                    graphql_name: "Status".to_owned(),
                    values: vec!["ACTIVE".to_owned()],
                    comment: None,
                },
                EnumModel {
                    rule_name: "project.status".to_owned(),
                    graphql_name: "Status".to_owned(),
                    values: vec!["OPEN".to_owned(), "CLOSED".to_owned()],
                    comment: None,
                },
            ],
            tables: vec![
                TableModel {
                    name: "account".to_owned(),
                    comment: None,
                    columns: vec![ColumnModel::scalar("id", "bigint", false), first],
                },
                TableModel {
                    name: "project".to_owned(),
                    comment: None,
                    columns: vec![ColumnModel::scalar("id", "bigint", false), second],
                },
            ],
        };
        let error = Generator
            .render(&base_config(), &schema)
            .unwrap_err()
            .to_string();
        assert!(error.contains("incompatible database enums"), "{error}");
    }

    #[test]
    fn explicit_include_restores_fk_and_exclude_wins() {
        let mut config = base_config();
        config.columns.include = vec!["work_item\\.committee_id".to_owned()];
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap()
                .contains("committeeId")
        );

        config.columns.exclude = vec!["work_item\\.committee_id".to_owned()];
        assert!(
            !Generator
                .render(&config, &schema())
                .unwrap()
                .contains("committeeId")
        );
    }

    #[test]
    fn nullable_and_non_null_override_and_conflict() {
        let mut config = base_config();
        config.types.nullable = vec!["work_item\\.id".to_owned()];
        config.types.non_null = vec!["work_item\\.display_name".to_owned()];
        let output = Generator.render(&config, &schema()).unwrap();
        assert!(output.contains("    id: UUID\n"));
        assert!(output.contains("    displayName: String!"));

        config.types.non_null.push("work_item\\.id".to_owned());
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap_err()
                .to_string()
                .contains("both")
        );
    }

    #[test]
    fn type_mapping_and_column_override_work() {
        let mut schema = schema();
        schema.tables[0]
            .columns
            .push(ColumnModel::scalar("payload", "custom_payload", true));
        let mut config = base_config();
        config.types.sql = BTreeMap::from([("custom_payload".to_owned(), "JSON".to_owned())]);
        assert!(
            Generator
                .render(&config, &schema)
                .unwrap()
                .contains("payload: JSON")
        );

        config.types.overrides = vec![TypeOverrideConfig {
            pattern: "work_item\\.payload".to_owned(),
            graphql: "String".to_owned(),
        }];
        assert!(
            Generator
                .render(&config, &schema)
                .unwrap()
                .contains("payload: String")
        );
    }

    #[test]
    fn unknown_type_fails_closed() {
        let mut schema = schema();
        schema.tables[0]
            .columns
            .push(ColumnModel::scalar("payload", "custom_payload", true));
        let error = Generator
            .render(&base_config(), &schema)
            .unwrap_err()
            .to_string();
        assert!(error.contains("custom_payload"));
        assert!(error.contains("columns.exclude"));
    }

    #[test]
    fn stale_rules_fail_closed() {
        let mut config = base_config();
        config.columns.exclude = vec!["work_item\\.renamed_away".to_owned()];
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap_err()
                .to_string()
                .contains("matches nothing")
        );

        let mut config = base_config();
        config.tables.exclude = vec!["renamed_away".to_owned()];
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap_err()
                .to_string()
                .contains("matches nothing")
        );

        let mut config = base_config();
        config.enums.include = vec!["renamed_away".to_owned()];
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap_err()
                .to_string()
                .contains("matches nothing")
        );
    }

    #[test]
    fn table_include_is_required_and_exclude_wins() {
        let mut config = base_config();
        config.tables.include.clear();
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap_err()
                .to_string()
                .contains("tables.include")
        );

        let mut config = base_config();
        config.tables.exclude = vec!["work_item".to_owned()];
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap_err()
                .to_string()
                .contains("select no table")
        );
    }

    #[test]
    fn enum_selection_modes_work() {
        let mut config = base_config();
        config.enums.include_unreferenced = true;
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap()
                .contains("enum Unused")
        );

        let mut config = base_config();
        config.enums.include = vec!["unused".to_owned()];
        assert!(
            Generator
                .render(&config, &schema())
                .unwrap()
                .contains("enum Unused")
        );
    }

    #[test]
    fn collisions_fail() {
        let mut schema = schema();
        schema.tables.push(TableModel {
            name: "work__item".to_owned(),
            comment: None,
            columns: vec![ColumnModel::scalar("id", "uuid", false)],
        });
        assert!(
            Generator
                .render(&base_config(), &schema)
                .unwrap_err()
                .to_string()
                .contains("more than one object")
        );
    }

    #[test]
    fn field_collision_fails() {
        let mut schema = schema();
        schema.tables[0]
            .columns
            .push(ColumnModel::scalar("display__name", "text", true));
        assert!(
            Generator
                .render(&base_config(), &schema)
                .unwrap_err()
                .to_string()
                .contains("two columns")
        );
    }

    #[test]
    fn verify_match_missing_and_different() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = base_config();
        config.generator.output = temp.path().join("nested/entities.graphqls");

        assert!(matches!(
            Generator.verify(&config, &schema()).unwrap(),
            Verification::Missing { .. }
        ));
        Generator.generate(&config, &schema()).unwrap();
        assert!(matches!(
            Generator.verify(&config, &schema()).unwrap(),
            Verification::Match { .. }
        ));
        fs::write(&config.generator.output, "wrong\n").unwrap();
        match Generator.verify(&config, &schema()).unwrap() {
            Verification::Different {
                line,
                expected,
                actual,
                ..
            } => {
                assert_eq!(line, 1);
                assert!(expected.is_some());
                assert_eq!(actual.as_deref(), Some("wrong"));
            }
            other => panic!("unexpected verification: {other:?}"),
        }
    }
}
