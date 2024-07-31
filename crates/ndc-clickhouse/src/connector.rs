pub mod handler;
pub mod state;

use std::{collections::BTreeMap, env, path::Path, str::FromStr};
use tokio::fs;

use async_trait::async_trait;
use ndc_sdk::{
    connector::{
        Connector, ConnectorSetup, ExplainError, FetchMetricsError, HealthError,
        InitializationError, InvalidNode, InvalidNodes, KeyOrIndex, LocatedError, MutationError,
        ParseError, QueryError, SchemaError,
    },
    json_response::JsonResponse,
    models,
};

use self::state::ServerState;
use common::{
    clickhouse_parser::{datatype::ClickHouseDataType, parameterized_query::ParameterizedQuery},
    config::{ConnectionConfig, ParameterizedQueryConfig, ServerConfig, TableConfig, TableType},
    config_file::{
        ParameterizedQueryConfigFile, ReturnType, ServerConfigFile, TableConfigFile,
        CONFIG_FILE_NAME,
    },
};

#[derive(Debug, Clone, Default)]
pub struct ClickhouseConnector;

#[async_trait]
impl ConnectorSetup for ClickhouseConnector {
    type Connector = Self;

    async fn parse_configuration(
        &self,
        configuration_dir: impl AsRef<Path> + Send,
    ) -> Result<<Self as Connector>::Configuration, ParseError> {
        read_server_config(configuration_dir).await
    }

    async fn try_init_state(
        &self,
        configuration: &<Self as Connector>::Configuration,
        _metrics: &mut prometheus::Registry,
    ) -> Result<<Self as Connector>::State, InitializationError> {
        Ok(ServerState::new(configuration))
    }
}

#[async_trait]
impl Connector for ClickhouseConnector {
    type Configuration = ServerConfig;
    type State = ServerState;

    fn fetch_metrics(
        _configuration: &Self::Configuration,
        _state: &Self::State,
    ) -> Result<(), FetchMetricsError> {
        Ok(())
    }

    async fn health_check(
        configuration: &Self::Configuration,
        state: &Self::State,
    ) -> Result<(), HealthError> {
        let client = state
            .client(configuration)
            .await
            .map_err(|err| HealthError::Other(err.to_string().into()))?;

        common::client::ping(&client, &configuration.connection)
            .await
            .map_err(|err| HealthError::Other(err.to_string().into()))?;

        Ok(())
    }

    async fn get_capabilities() -> JsonResponse<models::CapabilitiesResponse> {
        JsonResponse::Value(handler::capabilities())
    }

    async fn get_schema(
        configuration: &Self::Configuration,
    ) -> Result<JsonResponse<models::SchemaResponse>, SchemaError> {
        handler::schema(configuration).await
    }

    async fn query_explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>, ExplainError> {
        handler::explain(configuration, state, request).await
    }

    async fn mutation_explain(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>, ExplainError> {
        Err(ExplainError::UnsupportedOperation(
            "mutation explain not supported".to_string(),
        ))
    }

    async fn mutation(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::MutationResponse>, MutationError> {
        Err(MutationError::UnsupportedOperation(
            "mutation not supported".to_string(),
        ))
    }

    async fn query(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::QueryResponse>, QueryError> {
        handler::query(configuration, state, request).await
    }
}

/// read server configuration from env var
pub async fn read_server_config(
    configuration_dir: impl AsRef<Path> + Send,
) -> Result<ServerConfig, ParseError> {
    let connection = get_connection_config()?;

    let file_path = configuration_dir.as_ref().join(CONFIG_FILE_NAME);

    let config_file = fs::read_to_string(&file_path)
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => {
                ParseError::CouldNotFindConfiguration(file_path.to_owned())
            }
            _ => ParseError::IoError(err),
        })?;

    let config = serde_json::from_str::<ServerConfigFile>(&config_file).map_err(|err| {
        ParseError::ParseError(LocatedError {
            file_path: file_path.to_owned(),
            line: err.line(),
            column: err.column(),
            message: err.to_string(),
        })
    })?;

    let table_types = config
        .tables
        .iter()
        .map(|(table_alias, table_config)| {
            let table_type = validate_and_parse_return_type(
                &table_config.return_type,
                &config,
                &file_path,
                &["tables", &table_alias, "return_type"],
            )?
            .map(|columns| {
                (
                    table_alias.to_owned(),
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
                &["query", &query_alias, "return_type"],
            )?
            .map(|columns| {
                (
                    query_alias.to_owned(),
                    TableType {
                        comment: query_config.comment.to_owned(),
                        columns,
                    },
                )
            });

            Ok(table_type)
        }))
        .filter_map(|table_type| table_type.transpose())
        .collect::<Result<_, ParseError>>()?;

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
                        ReturnType::Definition { .. } => table_alias.to_owned(),
                        ReturnType::TableReference {
                            table_name: target_alias,
                        }
                        | ReturnType::QueryReference {
                            query_name: target_alias,
                        } => target_alias.to_owned(),
                    },
                    arguments: table_config
                        .arguments
                        .iter()
                        .map(|(name, r#type)| {
                            let data_type =
                                ClickHouseDataType::from_str(r#type).map_err(|_err| {
                                    ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                                        file_path: file_path.to_owned(),
                                        node_path: vec![
                                            KeyOrIndex::Key("tables".to_string()),
                                            KeyOrIndex::Key(table_alias.to_owned()),
                                            KeyOrIndex::Key("arguments".to_string()),
                                            KeyOrIndex::Key(name.to_owned()),
                                        ],
                                        message: "Unable to parse data type".to_string(),
                                    }]))
                                })?;

                            Ok((name.to_owned(), data_type))
                        })
                        .collect::<Result<_, ParseError>>()?,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, ParseError>>()?;

    let mut queries = BTreeMap::new();

    for (query_alias, query_config) in config.queries.clone() {
        let query_file_path = configuration_dir.as_ref().join(&query_config.file);
        let file_content = fs::read_to_string(&query_file_path).await.map_err(|err| {
            if let std::io::ErrorKind::NotFound = err.kind() {
                ParseError::CouldNotFindConfiguration(query_file_path.to_owned())
            } else {
                ParseError::IoError(err)
            }
        })?;

        let query = ParameterizedQuery::from_str(&file_content).map_err(|err| {
            ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                file_path: query_file_path.clone(),
                node_path: vec![
                    KeyOrIndex::Key("queries".to_string()),
                    KeyOrIndex::Key(query_alias.clone()),
                ],
                message: format!("Unable to parse parameterized query: {}", err),
            }]))
        })?;

        let query_definition = ParameterizedQueryConfig {
            exposed_as: query_config.exposed_as.to_owned(),
            comment: query_config.comment.to_owned(),
            query,
            return_type: match query_config.return_type {
                ReturnType::Definition { .. } => query_alias.to_owned(),
                ReturnType::TableReference {
                    table_name: target_alias,
                }
                | ReturnType::QueryReference {
                    query_name: target_alias,
                } => target_alias.to_owned(),
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

fn validate_and_parse_return_type(
    return_type: &ReturnType,
    config: &ServerConfigFile,
    file_path: &Path,
    node_path: &[&str],
) -> Result<Option<BTreeMap<String, ClickHouseDataType>>, ParseError> {
    let get_node_path = |extra_segments: &[&str]| {
        node_path
            .iter()
            .chain(extra_segments.iter())
            .map(|s| KeyOrIndex::Key(s.to_string()))
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
                    Err(ParseError::ValidateError(InvalidNodes(vec![
                        InvalidNode {
                            file_path: file_path.to_path_buf(),
                            node_path: get_node_path(&["table_name"]),
                            message: format!(
                            "Invalid reference: referenced table {} which does not have a return type definition",
                            table_name,
                        ),
                        },
                    ])))
                }
                None => {
                    Err(ParseError::ValidateError(InvalidNodes(vec![
                        InvalidNode {
                            file_path: file_path.to_path_buf(),
                            node_path: get_node_path(&["table_name"]),
                            message: format!(
                            "Orphan reference: cannot find referenced table {}",
                            table_name,
                        ),
                        },
                    ])))
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
                    Err(ParseError::ValidateError(InvalidNodes(vec![
                        InvalidNode {
                            file_path: file_path.to_path_buf(),
                            node_path: get_node_path(&["query_name"]),
                            message: format!(
                                "Invalid reference: referenced query {} which does not have a return type definition",
                            query_name,
                        ),
                        },
                    ])))
                }
                None => {
                    Err(ParseError::ValidateError(InvalidNodes(vec![
                        InvalidNode {
                            file_path: file_path.to_path_buf(),
                            node_path: get_node_path(&["query_name"]),
                            message: format!(
                                "Orphan reference: cannot find referenced query {}",
                            query_name,
                        ),
                        },
                    ])))
                }
            }
        }
        ReturnType::Definition { columns } => Ok(Some(

            columns
            .iter()
            .map(|(field_alias, field_type)| {
                let data_type = ClickHouseDataType::from_str(field_type).map_err(|err| {
                    ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                        file_path: file_path.to_path_buf(),
                        node_path: get_node_path(&["columns", field_alias]),
                        message: format!(
                            "Unable to parse data type \"{}\": {}",
                            field_type, err
                        ),
                    }]))
                })?;
                Ok((field_alias.to_owned(), data_type))
            })
            .collect::<Result<BTreeMap<String, ClickHouseDataType>, ParseError>>()?

        ))
    }
}
