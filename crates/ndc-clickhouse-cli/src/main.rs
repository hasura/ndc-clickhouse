use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand, ValueEnum};
use config::{
    ColumnConfig, ConnectionConfig, PrimaryKey, ServerConfigFile, TableConfig, CONFIG_FILE_NAME,
};
use database_introspection::{introspect_database, ColumnInfo, TableInfo};
use tokio::fs;
mod database_introspection;

#[derive(Parser)]
struct CliArgs {
    /// The PAT token which can be used to make authenticated calls to Hasura Cloud
    #[arg(long = "ddn-pat", value_name = "PAT", env = "HASURA_PLUGIN_DDN_PAT")]
    ddn_pat: Option<String>,
    /// If the plugins are sending any sort of telemetry back to Hasura, it should be disabled if this is true.
    #[arg(long = "disable-telemetry", env = "HASURA_PLUGIN_DISABLE_TELEMETRY")]
    disable_telemetry: bool,
    /// A UUID for every unique user. Can be used in telemetry
    #[arg(
        long = "instance-id",
        value_name = "ID",
        env = "HASURA_PLUGIN_INSTANCE_ID"
    )]
    instance_id: Option<String>,
    /// A UUID unique to every invocation of Hasura CLI
    #[arg(
        long = "execution-id",
        value_name = "ID",
        env = "HASURA_PLUGIN_EXECUTION_ID"
    )]
    execution_id: Option<String>,
    #[arg(
        long = "log-level",
        value_name = "LEVEL",
        env = "HASURA_PLUGIN_LOG_LEVEL",
        default_value = "info",
        ignore_case = true
    )]
    log_level: LogLevel,
    /// Fully qualified path to the context directory of the connector
    #[arg(
        long = "connector-context-path",
        value_name = "PATH",
        env = "HASURA_PLUGIN_CONNECTOR_CONTEXT_PATH"
    )]
    context_path: Option<PathBuf>,
    #[arg(long = "clickhouse-url", value_name = "URL", env = "CLICKHOUSE_URL")]
    clickhouse_url: String,
    #[arg(long = "clickhouse-username", value_name = "USERNAME", env = "CLICKHOUSE_USERNAME", default_value_t = String::from("default"))]
    clickhouse_username: String,
    #[arg(
        long = "clickhouse-password",
        value_name = "PASSWORD",
        env = "CLICKHOUSE_PASSWORD"
    )]
    clickhouse_password: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Subcommand)]
enum Command {
    Init {},
    Update {},
    Validate {},
    Watch {},
}

#[derive(Clone, ValueEnum)]
enum LogLevel {
    Panic,
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

struct Context {
    context_path: PathBuf,
    connection: ConnectionConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();

    let context_path = match args.context_path {
        None => env::current_dir()?,
        Some(path) => path,
    };

    let connection = ConnectionConfig {
        url: args.clickhouse_url,
        username: args.clickhouse_username,
        password: args.clickhouse_password,
    };

    let context = Context {
        context_path,
        connection,
    };

    match args.command {
        Command::Init {} => {
            update_tables_config(&context.context_path, &context.connection).await?;
        }
        Command::Update {} => {
            update_tables_config(&context.context_path, &context.connection).await?;
        }
        Command::Validate {} => {
            todo!("implement validate command")
        }
        Command::Watch {} => {
            todo!("implement watch command")
        }
    }

    Ok(())
}

pub async fn update_tables_config(
    configuration_dir: impl AsRef<Path> + Send,
    connection_config: &ConnectionConfig,
) -> Result<(), Box<dyn Error>> {
    let table_infos = introspect_database(connection_config).await?;

    let file_path = configuration_dir.as_ref().join(CONFIG_FILE_NAME);

    let old_config = match fs::read_to_string(&file_path).await {
        Ok(file) => serde_json::from_str(&file)
            .map_err(|err| format!("Error parsing {CONFIG_FILE_NAME}: {err}")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(ServerConfigFile::default()),
        Err(_) => Err(format!("Error reading {CONFIG_FILE_NAME}")),
    }?;

    let tables = table_infos
        .iter()
        .map(|table| {
            let old_table_config = get_old_table_config(table, &old_config.tables);

            TableConfig {
                name: table.table_name.to_owned(),
                schema: table.table_schema.to_owned(),
                alias: get_table_alias(table, &old_table_config),
                primary_key: table.primary_key.as_ref().map(|primary_key| PrimaryKey {
                    name: primary_key.to_owned(),
                    columns: table
                        .columns
                        .iter()
                        .filter_map(|column| {
                            if column.is_in_primary_key {
                                Some(get_column_alias(
                                    column,
                                    &get_old_column_config(column, &old_table_config),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect(),
                }),
                columns: table
                    .columns
                    .iter()
                    .map(|column| ColumnConfig {
                        name: column.column_name.to_owned(),
                        alias: get_column_alias(
                            column,
                            &get_old_column_config(column, &old_table_config),
                        ),
                        data_type: column.data_type.to_owned(),
                    })
                    .collect(),
            }
        })
        .collect();

    let config = ServerConfigFile { tables };

    fs::write(&file_path, serde_json::to_string(&config)?).await?;

    Ok(())
}

fn get_old_table_config<'a>(
    table: &TableInfo,
    old_tables: &'a [TableConfig],
) -> Option<&'a TableConfig> {
    old_tables.iter().find(|old_table| {
        old_table.name == table.table_name && old_table.schema == table.table_schema
    })
}

fn get_old_column_config<'a>(
    column: &ColumnInfo,
    old_table: &Option<&'a TableConfig>,
) -> Option<&'a ColumnConfig> {
    old_table
        .map(|old_table| {
            old_table
                .columns
                .iter()
                .find(|old_column| old_column.name == column.column_name)
        })
        .flatten()
}

fn get_table_alias(table: &TableInfo, old_table: &Option<&TableConfig>) -> String {
    // to preserve any customization, aliases are kept throught updates
    if let Some(old_table) = old_table {
        old_table.alias.to_owned()
    } else if table.table_schema == "default" {
        table.table_name.to_owned()
    } else {
        format!("{}_{}", table.table_schema, table.table_name)
    }
}

fn get_column_alias(column: &ColumnInfo, old_column: &Option<&ColumnConfig>) -> String {
    // to preserve any customization, aliases are kept throught updates
    if let Some(old_column) = old_column {
        old_column.alias.to_owned()
    } else {
        column.column_name.to_owned()
    }
}
