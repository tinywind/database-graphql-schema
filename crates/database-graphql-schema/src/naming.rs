use heck::{ToLowerCamelCase, ToSnakeCase, ToUpperCamelCase};

use crate::{Error, Result};

const RESERVED_TYPES: &[&str] = &[
    "Int",
    "Float",
    "String",
    "Boolean",
    "ID",
    "Query",
    "Mutation",
    "Subscription",
];

pub fn type_name(database_name: &str) -> String {
    database_name.to_upper_camel_case()
}

pub fn field_name(column_name: &str) -> String {
    column_name.to_lower_camel_case()
}

pub fn seaorm_field_name(column_name: &str) -> String {
    column_name.to_snake_case()
}

pub(crate) fn validate_type_name(name: &str, source: &str) -> Result<()> {
    validate_name(name, source)?;
    if RESERVED_TYPES.contains(&name) {
        return Err(Error::Generation(format!(
            "{source} maps to the GraphQL name '{name}', which is reserved; rename the object or exclude it"
        )));
    }
    Ok(())
}

pub(crate) fn validate_field_name(name: &str, source: &str) -> Result<()> {
    validate_name(name, source)
}

pub(crate) fn validate_enum_value(value: &str, source: &str) -> Result<()> {
    if !valid_graphql_name(value) || matches!(value, "true" | "false" | "null") {
        return Err(Error::Generation(format!(
            "{source} has the literal '{value}', which is not a valid GraphQL enum value"
        )));
    }
    Ok(())
}

fn validate_name(name: &str, source: &str) -> Result<()> {
    if !valid_graphql_name(name) || name.starts_with("__") {
        return Err(Error::Generation(format!(
            "{source} maps to '{name}', which is not a valid GraphQL name"
        )));
    }
    Ok(())
}

fn valid_graphql_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_jooq_and_seaorm_baseline_cases() {
        for (db, expected) in [
            ("user", "User"),
            ("access_log", "AccessLog"),
            ("board_type", "BoardType"),
            ("USER_STATUS", "UserStatus"),
            ("user_feed_board_setting", "UserFeedBoardSetting"),
        ] {
            assert_eq!(type_name(db), expected);
        }
        for (db, expected) in [
            ("id", "id"),
            ("board_type", "boardType"),
            ("cohort_number", "cohortNumber"),
            ("attestation_client_data_json", "attestationClientDataJson"),
            ("uv_initialized", "uvInitialized"),
        ] {
            assert_eq!(field_name(db), expected);
        }
        assert_eq!(seaorm_field_name("CreatedAt"), "created_at");
    }

    #[test]
    fn reserved_and_invalid_names_fail() {
        for name in ["Query", "Mutation", "String", "ID", "__Meta", "2fa"] {
            assert!(validate_type_name(name, "test").is_err(), "{name}");
        }
        for name in ["__typename", "1st"] {
            assert!(validate_field_name(name, "test").is_err(), "{name}");
        }
        for value in ["true", "false", "null", "in-progress", ""] {
            assert!(validate_enum_value(value, "test").is_err(), "{value}");
        }
    }
}
