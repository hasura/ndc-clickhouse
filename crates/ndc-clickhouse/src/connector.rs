pub mod handler;
pub mod setup;
pub mod state;

use self::state::ServerState;
use async_trait::async_trait;
use common::{capabilities::capabilities, config::ServerConfig, schema::schema_response};
use http::StatusCode;
use ndc_sdk::{
    connector::{Connector, ErrorResponse, Result},
    json_response::JsonResponse,
    models,
};

#[derive(Debug, Clone, Default)]
pub struct ClickhouseConnector;

#[async_trait]
impl Connector for ClickhouseConnector {
    type Configuration = ServerConfig;
    type State = ServerState;

    fn fetch_metrics(_configuration: &Self::Configuration, _state: &Self::State) -> Result<()> {
        Ok(())
    }

    async fn get_capabilities() -> models::Capabilities {
        capabilities()
    }

    async fn get_schema(
        configuration: &Self::Configuration,
    ) -> Result<JsonResponse<models::SchemaResponse>> {
        Ok(JsonResponse::Value(schema_response(configuration)))
    }

    async fn query_explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>> {
        handler::explain(configuration, state, request).await
    }

    async fn mutation_explain(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>> {
        Err(ErrorResponse::new(
            StatusCode::NOT_IMPLEMENTED,
            "mutation explain not supported".to_string(),
            serde_json::Value::Null,
        ))
    }

    async fn mutation(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::MutationResponse>> {
        Err(ErrorResponse::new(
            StatusCode::NOT_IMPLEMENTED,
            "mutation not supported".to_string(),
            serde_json::Value::Null,
        ))
    }

    async fn query(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::QueryResponse>> {
        handler::query(configuration, state, request).await
    }
}
