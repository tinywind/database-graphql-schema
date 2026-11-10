#![forbid(unsafe_code)]

use std::{ffi::OsString, fs, path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};
use database_graphql_schema::{
    Config, Verification, generate_from_database, render_from_database, verify_from_database,
};

#[derive(Debug, Parser)]
#[command(
    name = "cargo graphql-schema",
    bin_name = "cargo graphql-schema",
    version,
    about = "Generate and verify GraphQL entity SDL from relational database schemas"
)]
struct Cli {
    #[arg(long, global = true, default_value = "graphql-schema.toml")]
    config: PathBuf,

    /// Database URL. Prefer --database-url-file or an environment variable so secrets do not enter shell history.
    #[arg(long, global = true)]
    database_url: Option<String>,

    /// Read the database URL from a file.
    #[arg(long, global = true)]
    database_url_file: Option<PathBuf>,

    /// Override the environment variable named by database.url_env.
    #[arg(long, global = true)]
    database_url_env: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate the canonical SDL file.
    Generate,
    /// Regenerate in memory and fail if the output is missing or different.
    Verify,
    /// Print canonical SDL to stdout without changing the output file.
    Print,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let cli = Cli::parse_from(normalized_args());
    let config = Config::from_path(&cli.config).map_err(|error| error.to_string())?;
    let database_url = resolve_database_url(&cli, &config)?;

    match cli.command {
        Command::Generate => {
            let generated = generate_from_database(&config, &database_url)
                .await
                .map_err(|error| error.to_string())?;
            eprintln!(
                "generated {} ({} types, {} enums)",
                generated.file.display(),
                generated.type_count,
                generated.enum_count
            );
        }
        Command::Verify => match verify_from_database(&config, &database_url)
            .await
            .map_err(|error| error.to_string())?
        {
            Verification::Match { file } => {
                eprintln!("{} matches the database schema", file.display());
            }
            Verification::Missing { file } => {
                return Err(format!(
                    "{} does not exist; run cargo graphql-schema generate --config {}",
                    file.display(),
                    cli.config.display()
                ));
            }
            Verification::Different {
                file,
                line,
                expected,
                actual,
            } => {
                return Err(format!(
                    "{} differs from the database schema at line {line}\n  expected: {}\n  actual:   {}\nrun cargo graphql-schema generate --config {}",
                    file.display(),
                    expected.as_deref().unwrap_or("<end of file>"),
                    actual.as_deref().unwrap_or("<end of file>"),
                    cli.config.display()
                ));
            }
        },
        Command::Print => {
            let rendered = render_from_database(&config, &database_url)
                .await
                .map_err(|error| error.to_string())?;
            print!("{rendered}");
        }
    }
    Ok(())
}

fn resolve_database_url(cli: &Cli, config: &Config) -> Result<String, String> {
    if let Some(url) = &cli.database_url {
        return non_blank_url(url.clone(), "--database-url");
    }
    if let Some(path) = &cli.database_url_file {
        let url = fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read database URL file {}: {error}",
                path.display()
            )
        })?;
        return non_blank_url(url.trim().to_owned(), "--database-url-file");
    }

    let variable = cli
        .database_url_env
        .as_deref()
        .unwrap_or(&config.database.url_env);
    let value = std::env::var(variable)
        .map_err(|_| format!("database URL is not set; define environment variable {variable}, use --database-url-file, or pass --database-url"))?;
    non_blank_url(value, variable)
}

fn non_blank_url(value: String, source: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err(format!("database URL from {source} is blank"))
    } else {
        Ok(value)
    }
}

fn normalized_args() -> Vec<OsString> {
    let mut args: Vec<OsString> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "graphql-schema") {
        args.remove(1);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_blank_url_rejects_blank() {
        assert!(non_blank_url(" ".to_owned(), "TEST_DATABASE_URL").is_err());
    }
}
