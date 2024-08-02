pub mod handler;
pub mod setup;
pub mod state;

use self::state::ServerState;
use async_trait::async_trait;
use common::config::ServerConfig;
use ndc_sdk::{
    connector::{
        Connector, ExplainError, FetchMetricsError, HealthError, MutationError, QueryError,
        SchemaError,
    },
    json_response::JsonResponse,
    models,
};

#[derive(Debug, Clone, Default)]
pub struct ClickhouseConnector;

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
            .map_err(HealthError::new)?;

        common::client::ping(&client, &configuration.connection)
            .await
            .map_err(HealthError::new)?;

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
        Err(ExplainError::new_unsupported_operation(
            &"mutation explain not supported",
        ))
    }

    async fn mutation(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::MutationResponse>, MutationError> {
        Err(MutationError::new_unsupported_operation(
            &"mutation not supported",
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
