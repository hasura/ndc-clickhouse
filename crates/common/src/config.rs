use crate::{
    clickhouse_parser::{datatype::ClickHouseDataType, parameterized_query::ParameterizedQuery},
    config_file::{
        ColumnDefinition, MaybeClickhouseDataType, ParameterizedQueryConfigFile,
        ParameterizedQueryExposedAs, PrimaryKey, ReturnType, ServerConfigFile, TableConfigFile,
        CONFIG_FILE_NAME,
    },
    format::display_period_separated,
};
use ndc_models::{ArgumentName, CollectionName, FieldName, ObjectTypeName};
use std::{
    collections::{BTreeMap, HashMap},
    env, io,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::fs;

#[derive(Debug, Clone)]
/// In memory, runtime configuration, built from the configuration file(s) and environment variables
pub struct ServerConfig {
    /// the connection part of the config is not part of the config file
    pub connection: ConnectionConfig,
    pub namespace_separator: String,
    pub table_types: BTreeMap<ObjectTypeName, TableType>,
    pub tables: BTreeMap<CollectionName, TableConfig>,
    pub queries: BTreeMap<CollectionName, ParameterizedQueryConfig>,
}

#[derive(Debug, Clone)]
pub struct TableType {
    pub comment: Option<String>,
    pub columns: BTreeMap<FieldName, ColumnType>,
}

#[derive(Debug, Clone)]
pub struct ColumnType {
    pub comment: Option<String>,
    pub data_type: ClickHouseDataType,
}

#[derive(Debug, Default, Clone)]
pub struct ConnectionConfig {
    pub username: String,
    pub password: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct TableConfig {
    /// The table name
    pub name: String,
    /// The table schema
    pub schema: String,
    /// Comments are sourced from the database table comment
    pub comment: Option<String>,
    pub primary_key: Option<PrimaryKey>,
    pub arguments: BTreeMap<ArgumentName, ClickHouseDataType>,
    // this key coresponds to a return type definition in the config table types
    pub return_type: ObjectTypeName,
}

#[derive(Debug, Clone)]
pub struct ParameterizedQueryConfig {
    pub exposed_as: ParameterizedQueryExposedAs,
    pub comment: Option<String>,
    pub query: ParameterizedQuery,
    pub return_type: ObjectTypeName,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("missing required environment variable: {0}")]
    MissingEnvironmentVariable(String),
    #[error("could not find configuration file: {0}")]
    FileNotFound(PathBuf),
    #[error("error processing configuration: {0}")]
    IoError(io::Error),
    #[error(
        "error parsing configuration: {file_path}, at line {line}, column {column}: {message}"
    )]
    ParseError {
        file_path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },
    #[error(
        "error validating configuration: {file_path}, at {}: {message}",
        display_period_separated(node_path)
    )]
    ValidateError {
        file_path: PathBuf,
        node_path: Vec<String>,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct ConfigurationEnvironment {
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

impl ConfigurationEnvironment {
    pub fn from_environment() -> Self {
        Self {
            url: env::var("CLICKHOUSE_URL").ok(),
            username: env::var("CLICKHOUSE_USERNAME").ok(),
            password: env::var("CLICKHOUSE_PASSWORD").ok(),
        }
    }
    pub fn from_simulated_environment(env: HashMap<String, String>) -> Self {
        Self {
            url: env.get("CLICKHOUSE_URL").cloned(),
            username: env.get("CLICKHOUSE_USERNAME").cloned(),
            password: env.get("CLICKHOUSE_PASSWORD").cloned(),
        }
    }
}

pub fn get_connection_configuration(
    env: &ConfigurationEnvironment,
) -> Result<ConnectionConfig, ConfigurationError> {
    let url = env
        .url
        .to_owned()
        .ok_or(ConfigurationError::MissingEnvironmentVariable(
            "CLICKHOUSE_URL".into(),
        ))?;
    let username =
        env.username
            .to_owned()
            .ok_or(ConfigurationError::MissingEnvironmentVariable(
                "CLICKHOUSE_USERNAME".into(),
            ))?;
    let password =
        env.password
            .to_owned()
            .ok_or(ConfigurationError::MissingEnvironmentVariable(
                "CLICKHOUSE_PASSWORD".into(),
            ))?;

    Ok(ConnectionConfig {
        url,
        username,
        password,
    })
}

pub async fn read_server_config(
    configuration_dir: &Path,
    environment: &ConfigurationEnvironment,
) -> Result<ServerConfig, ConfigurationError> {
    let file_path = configuration_dir.join(CONFIG_FILE_NAME);

    let connection = get_connection_configuration(environment)?;

    let config_file = fs::read_to_string(&file_path)
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => ConfigurationError::FileNotFound(file_path.to_owned()),
            _ => ConfigurationError::IoError(err),
        })?;

    let config = serde_json::from_str::<ServerConfigFile>(&config_file).map_err(|err| {
        ConfigurationError::ParseError {
            file_path: file_path.to_owned(),
            line: err.line(),
            column: err.column(),
            message: err.to_string(),
        }
    })?;

    let table_types = config
        .tables
        .iter()
        .map(|(table_alias, table_config)| {
            let table_type = validate_and_parse_return_type(
                &table_config.return_type,
                &config,
                &file_path,
                &["tables", table_alias.inner(), "return_type"],
            )?
            .map(|columns| {
                (
                    table_alias.to_string().into(),
                    TableType {
                        comment: table_config.comment.to_owned(),
                        columns,
                    },
                )
            });

            Ok(table_type)
        })
        .chain(config.queries.iter().map(|(query_alias, query_config)| {
            let table_type = validate_and_parse_return_type(
                &query_config.return_type,
                &config,
                &file_path,
                &["query", query_alias.inner(), "return_type"],
            )?
            .map(|columns| {
                (
                    query_alias.to_string().into(),
                    TableType {
                        comment: query_config.comment.to_owned(),
                        columns,
                    },
                )
            });

            Ok(table_type)
        }))
        .filter_map(|table_type| table_type.transpose())
        .collect::<Result<_, ConfigurationError>>()?;

    let tables = config
        .tables
        .iter()
        .map(|(table_alias, table_config)| {
            Ok((
                table_alias.clone(),
                TableConfig {
                    name: table_config.name.to_owned(),
                    schema: table_config.schema.to_owned(),
                    comment: table_config.comment.to_owned(),
                    primary_key: table_config.primary_key.to_owned(),
                    return_type: match &table_config.return_type {
                        ReturnType::Definition { .. } => table_alias.to_string().into(),
                        ReturnType::TableReference {
                            table_name: target_alias,
                        }
                        | ReturnType::QueryReference {
                            query_name: target_alias,
                        } => target_alias.to_string().into(),
                    },
                    arguments: table_config
                        .arguments
                        .iter()
                        .map(|(name, r#type)| {
                            let data_type =
                                ClickHouseDataType::from_str(r#type).map_err(|err| {
                                    ConfigurationError::ValidateError {
                                        file_path: file_path.to_owned(),
                                        node_path: vec![
                                            "tables".to_string(),
                                            table_alias.to_string(),
                                            "arguments".to_string(),
                                            name.to_string(),
                                        ],
                                        message: format!("Unable to parse data type: {err}"),
                                    }
                                })?;

                            Ok((name.to_owned(), data_type))
                        })
                        .collect::<Result<_, ConfigurationError>>()?,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, ConfigurationError>>()?;

    let mut queries = BTreeMap::new();

    for (query_alias, query_config) in config.queries.clone() {
        let query_file_path = configuration_dir.join(&query_config.file);
        let file_content =
            fs::read_to_string(&query_file_path)
                .await
                .map_err(|err| match err.kind() {
                    std::io::ErrorKind::NotFound => {
                        ConfigurationError::FileNotFound(query_file_path.to_owned())
                    }
                    _ => ConfigurationError::IoError(err),
                })?;

        let query = ParameterizedQuery::from_str(&file_content).map_err(|err| {
            ConfigurationError::ValidateError {
                file_path: query_file_path.clone(),
                node_path: vec!["queries".to_string(), query_alias.to_string()],
                message: format!("Unable to parse parameterized query: {}", err),
            }
        })?;

        let query_definition = ParameterizedQueryConfig {
            exposed_as: query_config.exposed_as.to_owned(),
            comment: query_config.comment.to_owned(),
            query,
            return_type: match query_config.return_type {
                ReturnType::Definition { .. } => query_alias.to_string().into(),
                ReturnType::TableReference {
                    table_name: target_alias,
                }
                | ReturnType::QueryReference {
                    query_name: target_alias,
                } => target_alias.to_string().into(),
            },
        };

        queries.insert(query_alias.to_owned(), query_definition);
    }

    let config = ServerConfig {
        connection,
        // hardcoding separator for now, to avoid prematurely exposing configuration options we may not want to keep
        // if we make this configurable, we must default to this separator when the option is not provided
        namespace_separator: ".".to_string(),
        table_types,
        tables,
        queries,
    };

    Ok(config)
}

fn validate_and_parse_return_type(
    return_type: &ReturnType,
    config: &ServerConfigFile,
    file_path: &Path,
    node_path: &[&str],
) -> Result<Option<BTreeMap<FieldName, ColumnType>>, ConfigurationError> {
    let get_node_path = |extra_segments: &[&str]| {
        node_path
            .iter()
            .chain(extra_segments.iter())
            .map(ToString::to_string)
            .collect()
    };
    match return_type {
        ReturnType::TableReference { table_name } => {
            match config.tables.get(table_name) {
                Some(TableConfigFile {
                    return_type: ReturnType::Definition { .. },
                    ..
                }) => Ok(None),
                Some(_) => {
                    Err(ConfigurationError::ValidateError { file_path: file_path.to_path_buf(), node_path: get_node_path(&["table_name"]), message: format!(
                        "Invalid reference: referenced table {} which does not have a return type definition",
                        table_name,
                    ), })
                }
                None => {
                    Err(ConfigurationError::ValidateError { file_path: file_path.to_path_buf(), node_path: get_node_path(&["table_name"]), message: format!(
                        "Orphan reference: cannot find referenced table {}",
                        table_name,
                    )})
                }
            }
        }
        ReturnType::QueryReference { query_name } => {
            match config.queries.get(query_name) {
                Some(ParameterizedQueryConfigFile {
                    return_type: ReturnType::Definition { .. },
                    ..
                }) => Ok(None),
                Some(_) => {
                    Err(ConfigurationError::ValidateError { file_path: file_path.to_path_buf(),
                        node_path: get_node_path(&["query_name"]),
                        message: format!(
                            "Invalid reference: referenced query {} which does not have a return type definition",
                        query_name,
                    ), })
                }
                None => {
                    Err(ConfigurationError::ValidateError { file_path: file_path.to_path_buf(),
                        node_path: get_node_path(&["query_name"]),
                        message: format!(
                            "Orphan reference: cannot find referenced query {}",
                        query_name,                    ), })
                }
            }
        }
        ReturnType::Definition { columns } => Ok(Some(
            columns
                .iter()
                .map(|(field_alias, field_info)| {
                    let (data_type, comment, node_path) = match field_info {
                        ColumnDefinition::ShortHand(data_type) => (data_type, &None, get_node_path(&["columns", field_alias.inner()])),
                        ColumnDefinition::LongForm { data_type, comment } => (data_type, comment, get_node_path(&["columns", field_alias.inner(), "data_type"])),
                    };
                    // set empty comment to None
                    let comment = comment.as_ref().filter(|comment| !comment.is_empty());
                    match data_type {
                        MaybeClickhouseDataType::Valid(data_type) => Ok((field_alias.to_owned(), ColumnType { data_type: data_type.to_owned(), comment: comment.cloned() })),
                        MaybeClickhouseDataType::Invalid(malformed_string) => Err(ConfigurationError::ValidateError { file_path: file_path.into(), node_path, message: format!("Invalid data type \"{malformed_string}\"") }),
                    }
                })
                .collect::<Result<BTreeMap<FieldName, ColumnType>, ConfigurationError>>()?,
        )),
    }
}
