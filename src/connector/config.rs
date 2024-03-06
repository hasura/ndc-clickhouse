use std::{
    env,
    error::Error,
    fs::{self, read_to_string},
    path::Path,
};

use ndc_sdk::connector::{LocatedError, ParseError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use self::database_introspection::{introspect_database, ColumnInfo, TableInfo};

mod database_introspection;

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfigFile {
    pub tables: Vec<TableConfig>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfig {
    /// the connection part of the config is not part of the config file
    pub connection: ConnectionConfig,
    pub tables: Vec<TableConfig>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConnectionConfig {
    pub username: String,
    pub password: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableConfig {
    pub name: String,
    pub schema: String,
    pub alias: String,
    pub primary_key: Option<PrimaryKey>,
    pub columns: Vec<ColumnConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PrimaryKey {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnConfig {
    pub name: String,
    pub alias: String,
    pub data_type: String,
}

const CONFIG_FILE_NAME: &str = "configuration.json";

/// read server configuration from env var
pub async fn read_server_config(
    configuration_dir: impl AsRef<Path> + Send,
) -> Result<ServerConfig, ParseError> {
    let connection = get_connection_config()?;

    let file_path = configuration_dir.as_ref().join(CONFIG_FILE_NAME);

    let config_file = read_to_string(&file_path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => ParseError::CouldNotFindConfiguration(file_path.to_owned()),
        _ => ParseError::IoError(err),
    })?;

    let ServerConfigFile { tables } = serde_json::from_str::<ServerConfigFile>(&config_file)
        .map_err(|err| {
            ParseError::ParseError(LocatedError {
                file_path,
                line: err.line(),
                column: err.column(),
                message: err.to_string(),
            })
        })?;

    Ok(ServerConfig { connection, tables })
}

pub async fn update_tables_config(
    configuration_dir: impl AsRef<Path> + Send,
    connection_config: &ConnectionConfig,
) -> Result<(), Box<dyn Error>> {
    let table_infos = introspect_database(connection_config).await?;

    let file_path = configuration_dir.as_ref().join(CONFIG_FILE_NAME);

    let old_config = match read_to_string(&file_path) {
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
                primary_key: table.primary_key.as_ref().map(|primary_key| {
                    PrimaryKey {
                        name: primary_key.to_owned(),
                        columns: table
                            .columns
                            .iter()
                            .filter_map(|column| {
                                if column.is_in_primary_key {
                                    // note: we should alias the column here.
                                    Some(get_column_alias(
                                        column,
                                        &get_old_column_config(column, &old_table_config),
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    }
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

    fs::write(&file_path, serde_json::to_string(&config)?)?;

    Ok(())
}

fn get_connection_config() -> Result<ConnectionConfig, ParseError> {
    // define what the new configuration will look like
    // assemble config from env vars and reading files in config directory
    let url = env::var("CLICKHOUSE_URL")
        .map_err(|_err| ParseError::Other("CLICKHOUSE_URL env var must be set".into()))?;
    let username = env::var("CLICKHOUSE_USERNAME")
        .map_err(|_err| ParseError::Other("CLICKHOUSE_USERNAME env var must be set".into()))?;
    let password = env::var("CLICKHOUSE_PASSWORD")
        .map_err(|_err| ParseError::Other("CLICKHOUSE_PASSWORD env var must be set".into()))?;

    Ok(ConnectionConfig {
        url,
        username,
        password,
    })
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

/// generate an updated configuration by introspecting the database
/// expects a generated_config.json file to exists with connection information.
/// If not such file exists, you can create one by using this template:
/// ```json
/// {
///   "connection": {
///     "username": "",
///     "password": "",
///     "url": ""
///   },
///   "tables": []
/// }
/// ```
#[tokio::test]
async fn update_config() -> Result<(), Box<dyn Error>> {
    let connection = get_connection_config()?;

    let configuration_dir = ".";

    update_tables_config(configuration_dir, &connection).await?;

    Ok(())
}
