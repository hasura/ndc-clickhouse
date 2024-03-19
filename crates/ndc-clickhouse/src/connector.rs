pub mod handler;
pub mod state;

use std::{env, path::Path};
use tokio::fs;

use async_trait::async_trait;
use ndc_sdk::{
    connector::{
        Connector, ConnectorSetup, ExplainError, FetchMetricsError, HealthError,
        InitializationError, LocatedError, MutationError, ParseError, QueryError, SchemaError,
    },
    json_response::JsonResponse,
    models,
};

use self::state::ServerState;
use config::{ConnectionConfig, ServerConfig, ServerConfigFile, CONFIG_FILE_NAME};

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

        client::ping(&client, &configuration.connection)
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
