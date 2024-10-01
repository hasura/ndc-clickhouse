use async_trait::async_trait;
use common::{
    clickhouse_parser::{datatype::ClickHouseDataType, parameterized_query::ParameterizedQuery},
    config::{ConnectionConfig, ParameterizedQueryConfig, ServerConfig, TableConfig, TableType},
    config_file::{
        ParameterizedQueryConfigFile, ReturnType, ServerConfigFile, TableConfigFile,
        CONFIG_FILE_NAME,
    },
};
use ndc_sdk::{
    connector::{
        self, Connector, ConnectorSetup, InvalidNode, InvalidNodes, KeyOrIndex, LocatedError,
        ParseError,
    },
    models::FieldName,
};
use std::{
    collections::{BTreeMap, HashMap},
    env,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::fs;

use super::{state::ServerState, ClickhouseConnector};
#[derive(Debug, Clone)]
pub struct ClickhouseConnectorSetup {
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

#[async_trait]
impl ConnectorSetup for ClickhouseConnectorSetup {
    type Connector = ClickhouseConnector;

    async fn parse_configuration(
        &self,
        configuration_dir: impl AsRef<Path> + Send,
    ) -> connector::Result<<Self::Connector as Connector>::Configuration> {
        // we wrap read_server_config so the ParseError is implicitly converted into an ErrorResponse
        Ok(self.read_server_config(configuration_dir).await?)
    }

    async fn try_init_state(
        &self,
        configuration: &<Self::Connector as Connector>::Configuration,
        _metrics: &mut prometheus::Registry,
    ) -> connector::Result<<Self::Connector as Connector>::State> {
        Ok(ServerState::new(configuration))
    }
}

impl Default for ClickhouseConnectorSetup {
    fn default() -> Self {
        Self {
            url: env::var("CLICKHOUSE_URL").ok(),
            username: env::var("CLICKHOUSE_USERNAME").ok(),
            password: env::var("CLICKHOUSE_PASSWORD").ok(),
        }
    }
}

impl ClickhouseConnectorSetup {
    pub fn new_from_env(env: HashMap<String, String>) -> Self {
        Self {
            url: env.get("CLICKHOUSE_URL").cloned(),
            username: env.get("CLICKHOUSE_USERNAME").cloned(),
            password: env.get("CLICKHOUSE_PASSWORD").cloned(),
        }
    }
    pub async fn read_server_config(
        &self,
        configuration_dir: impl AsRef<Path> + Send,
    ) -> Result<ServerConfig, ParseError> {
        let file_path = configuration_dir.as_ref().join(CONFIG_FILE_NAME);

        let connection = self.get_connection_config(&file_path)?;

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
                let table_type = self
                    .validate_and_parse_return_type(
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
                let table_type = self
                    .validate_and_parse_return_type(
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
                                    ClickHouseDataType::from_str(r#type).map_err(|_err| {
                                        ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                                            file_path: file_path.to_owned(),
                                            node_path: vec![
                                                KeyOrIndex::Key("tables".to_string()),
                                                KeyOrIndex::Key(table_alias.to_string()),
                                                KeyOrIndex::Key("arguments".to_string()),
                                                KeyOrIndex::Key(name.to_string()),
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
                        KeyOrIndex::Key(query_alias.to_string()),
                    ],
                    message: format!("Unable to parse parameterized query: {}", err),
                }]))
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
    fn get_connection_config(&self, file_path: &PathBuf) -> Result<ConnectionConfig, ParseError> {
        let url = self
            .url
            .to_owned()
            .ok_or(ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                file_path: file_path.to_owned(),
                node_path: vec![],
                message: "CLICKHOUSE_URL env var must be set".into(),
            }])))?;
        let username = self
            .username
            .to_owned()
            .ok_or(ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                file_path: file_path.to_owned(),
                node_path: vec![],
                message: "CLICKHOUSE_USERNAME env var must be set".into(),
            }])))?;
        let password = self
            .password
            .to_owned()
            .ok_or(ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                file_path: file_path.to_owned(),
                node_path: vec![],
                message: "CLICKHOUSE_PASSWORD env var must be set".into(),
            }])))?;

        Ok(ConnectionConfig {
            url,
            username,
            password,
        })
    }
    fn validate_and_parse_return_type(
        &self,
        return_type: &ReturnType,
        config: &ServerConfigFile,
        file_path: &Path,
        node_path: &[&str],
    ) -> Result<Option<BTreeMap<FieldName, ClickHouseDataType>>, ParseError> {
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
                        let data_type =
                            ClickHouseDataType::from_str(field_type).map_err(|err| {
                                ParseError::ValidateError(InvalidNodes(vec![InvalidNode {
                                    file_path: file_path.to_path_buf(),
                                    node_path: get_node_path(&["columns", field_alias.inner()]),
                                    message: format!(
                                        "Unable to parse data type \"{}\": {}",
                                        field_type, err
                                    ),
                                }]))
                            })?;
                        Ok((field_alias.to_owned(), data_type))
                    })
                    .collect::<Result<BTreeMap<FieldName, ClickHouseDataType>, ParseError>>()?,
            )),
        }
    }
}
