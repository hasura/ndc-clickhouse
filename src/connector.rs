pub mod client;
pub mod config;
pub mod handler;
pub mod state;

use std::{env, fs::read_to_string, path::Path};

use async_trait::async_trait;
use ndc_sdk::{
    connector::{
        Connector, ConnectorSetup, ExplainError, FetchMetricsError, HealthError,
        InitializationError, MutationError, ParseError, QueryError, SchemaError,
    },
    json_response::JsonResponse,
    models,
};

use self::{
    config::{read_server_config, ServerConfig},
    state::ServerState,
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
