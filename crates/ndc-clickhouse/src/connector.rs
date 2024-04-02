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
    clickhouse_parser::{
        self, datatype::ClickHouseDataType, parameterized_query::ParameterizedQuery,
    },
    config::{
        ColumnConfig, ConnectionConfig, ParameterizedQueryConfig, ParameterizedQueryConfigFile,
        ParameterizedQueryReturnType, ServerConfig, ServerConfigFile, TableConfig,
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
        handler::schema(configuration)
            .await
            .map(JsonResponse::Value)
    }

    async fn query_explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>, ExplainError> {
        handler::explain(configuration, state, request)
            .await
            .map(JsonResponse::Value)
            .map_err(|err| ExplainError::Other(err.to_string().into()))
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
        handler::query(configuration, state, request)
            .await
            .map(JsonResponse::Value)
            .map_err(|err| QueryError::Other(err.to_string().into()))
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

    let tables = config
        .tables
        .unwrap_or_default()
        .into_iter()
        .map(|(table_alias, table_config)| {
            Ok((
                table_alias.clone(),
                TableConfig {
                    name: table_config.name,
                    schema: table_config.schema,
                    comment: table_config.comment,
                    primary_key: table_config.primary_key,
                    columns: table_config
                        .columns
                        .into_iter()
                        .map(|(column_alias, column_config)| {
                            Ok((
                                column_alias.clone(),
                                ColumnConfig {
                                    name: column_config.name,
                                    data_type: ClickHouseDataType::from_str(
                                        &column_config.data_type,
                                    )
                                    .map_err(|_err| {
                                        ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                                            file_path: file_path.to_owned(),
                                            node_path: vec![
                                                KeyOrIndex::Key("tables".to_string()),
                                                KeyOrIndex::Key(table_alias.to_owned()),
                                                KeyOrIndex::Key("columns".to_string()),
                                                KeyOrIndex::Key(column_alias.to_owned()),
                                                KeyOrIndex::Key("data_type".to_string()),
                                            ],
                                            message: "Unable to parse data type".to_string(),
                                        }]))
                                    })?,
                                },
                            ))
                        })
                        .collect::<Result<_, ParseError>>()?,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, ParseError>>()?;

    let mut queries = BTreeMap::new();

    for (query_alias, query_config) in config.queries.clone().unwrap_or_default() {
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
            return_type: match &query_config.return_type {
                ParameterizedQueryReturnType::TableReference {
                    table_alias: target_alias,
                } => {
                    if tables.contains_key(target_alias) {
                        ParameterizedQueryReturnType::TableReference {
                            table_alias: target_alias.to_owned(),
                        }
                    } else {
                        return Err(ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                            file_path: file_path.clone(),
                            node_path: vec![
                                KeyOrIndex::Key("queries".to_owned()),
                                KeyOrIndex::Key(query_alias.to_owned()),
                                KeyOrIndex::Key("return_type".to_owned()),
                                KeyOrIndex::Key("alias".to_owned()),
                            ],
                            message: format!(
                                "Orphan reference: cannot table {} referenced by query {}",
                                target_alias, query_alias
                            ),
                        }])));
                    }
                }
                ParameterizedQueryReturnType::QueryReference {
                    query_alias: target_alias,
                } => match config
                    .queries
                    .as_ref()
                    .and_then(|queries| queries.get(target_alias))
                {
                    Some(ParameterizedQueryConfigFile {
                        return_type: ParameterizedQueryReturnType::Custom { .. },
                        ..
                    }) => ParameterizedQueryReturnType::QueryReference {
                        query_alias: target_alias.to_owned(),
                    },
                    Some(_) => {
                        return Err(ParseError::ValidateError(InvalidNodes(vec![
                                InvalidNode {
                                    file_path: file_path.clone(),
                                    node_path: vec![
                                        KeyOrIndex::Key("queries".to_owned()),
                                        KeyOrIndex::Key(query_alias.to_owned()),
                                        KeyOrIndex::Key("return_type".to_owned()),
                                        KeyOrIndex::Key("alias".to_owned()),
                                    ],
                                    message: format!(
                                        "Invalid reference: query {} referenced by query {} does not have a custom return type",
                                        target_alias, query_alias
                                    ),
                                },
                            ])));
                    }
                    None => {
                        return Err(ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                            file_path: file_path.clone(),
                            node_path: vec![
                                KeyOrIndex::Key("queries".to_owned()),
                                KeyOrIndex::Key(query_alias.to_owned()),
                                KeyOrIndex::Key("return_type".to_owned()),
                                KeyOrIndex::Key("alias".to_owned()),
                            ],
                            message: format!(
                                "Orphan reference: cannot table {} referenced by query {}",
                                target_alias, query_alias
                            ),
                        }])));
                    }
                },
                ParameterizedQueryReturnType::Custom { fields } => {
                    ParameterizedQueryReturnType::Custom {
                        fields: fields
                            .into_iter()
                            .map(|(field_alias, field_type)| {
                                let data_type =
                                    ClickHouseDataType::from_str(&field_type).map_err(|err| {
                                        ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                                            file_path: file_path.clone(),
                                            node_path: vec![
                                                KeyOrIndex::Key("queries".to_string()),
                                                KeyOrIndex::Key(query_alias.clone()),
                                                KeyOrIndex::Key("return_type".to_string()),
                                                KeyOrIndex::Key("fields".to_string()),
                                                KeyOrIndex::Key(field_alias.clone()),
                                            ],
                                            message: format!(
                                                "Unable to parse data type \"{}\": {}",
                                                field_type, err
                                            ),
                                        }]))
                                    })?;
                                Ok((field_alias.to_owned(), data_type))
                            })
                            .collect::<Result<_, ParseError>>()?,
                    }
                }
            },
        };

        queries.insert(query_alias.to_owned(), query_definition);
    }

    let config = ServerConfig {
        connection,
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
