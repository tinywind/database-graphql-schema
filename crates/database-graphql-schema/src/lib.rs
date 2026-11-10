#![forbid(unsafe_code)]

//! Deterministic, fail-closed GraphQL entity SDL generation from relational databases.
//!
//! The migrated database logical schema is the source of truth. This crate
//! intentionally generates only table scalar fields and native enums; it does
//! not infer relations, operations, authorization, or business projections.
//!
//! Supported metadata backends in 0.2.x are PostgreSQL, MySQL/MariaDB, and SQLite.

mod config;
mod error;
mod generator;
mod metadata;
mod model;
mod naming;
mod render;
mod rules;
mod types;

pub use config::{
    ColumnsConfig, Config, DatabaseConfig, DatabaseDriver, EnumsConfig, GeneratorConfig,
    TableConfig, TypeOverrideConfig, TypesConfig,
};
pub use error::{Error, Result};
pub use generator::{
    GeneratedSchema, Generator, Verification, generate_from_database, render_from_database,
    verify_from_database,
};
pub use metadata::{DatabaseMetadataReader, detect_database_kind};
pub use model::{ColumnModel, DatabaseKind, EnumModel, SchemaModel, TableModel};
pub use naming::{field_name, seaorm_field_name, type_name};
