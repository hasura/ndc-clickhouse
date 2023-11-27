pub mod client;
pub mod config;
pub mod handler;
pub mod state;

use async_trait::async_trait;
use ndc_sdk::{
    connector::{
        Connector, ExplainError, FetchMetricsError, HealthError, InitializationError,
        MutationError, QueryError, SchemaError, UpdateConfigurationError, ValidateError,
    },
    json_response::JsonResponse,
    models,
};

use self::{config::ServerConfig, state::ServerState};

#[derive(Debug, Clone, Default)]
pub struct ClickhouseConnector;

#[async_trait]
impl Connector for ClickhouseConnector {
    /// The type of unvalidated, raw configuration, as provided by the user.
    type RawConfiguration = ServerConfig;

    /// The type of validated configuration
    type Configuration = ServerConfig;

    /// The type of unserializable state
    type State = ServerState;

    fn make_empty_configuration() -> Self::RawConfiguration {
        ServerConfig::default()
    }

    async fn update_configuration(
        config: Self::RawConfiguration,
    ) -> Result<Self::RawConfiguration, UpdateConfigurationError> {
        handler::update_configuration(config).await
    }

    /// Validate the raw configuration provided by the user,
    /// returning a configuration error or a validated [`Connector::Configuration`].
    async fn validate_raw_configuration(
        configuration: Self::RawConfiguration,
    ) -> Result<Self::Configuration, ValidateError> {
        // todo: validate config.
        // todo: we should take an owned configuration here.
        Ok(configuration.to_owned())
    }

    /// Initialize the connector's in-memory state.
    ///
    /// For example, any connection pools, prepared queries,
    /// or other managed resources would be allocated here.
    ///
    /// In addition, this function should register any
    /// connector-specific metrics with the metrics registry.
    async fn try_init_state(
        configuration: &Self::Configuration,
        _metrics: &mut prometheus::Registry,
    ) -> Result<Self::State, InitializationError> {
        Ok(ServerState::new(configuration))
    }

    /// Update any metrics from the state
    ///
    /// Note: some metrics can be updated directly, and do not
    /// need to be updated here. This function can be useful to
    /// query metrics which cannot be updated directly, e.g.
    /// the number of idle connections in a connection pool
    /// can be polled but not updated directly.
    fn fetch_metrics(
        _configuration: &Self::Configuration,
        _state: &Self::State,
    ) -> Result<(), FetchMetricsError> {
        Ok(())
    }

    /// Check the health of the connector.
    ///
    /// For example, this function should check that the connector
    /// is able to reach its data source over the network.
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

    /// Get the connector's capabilities.
    ///
    /// This function implements the [capabilities endpoint](https://hasura.github.io/ndc-spec/specification/capabilities.html)
    /// from the NDC specification.
    async fn get_capabilities() -> JsonResponse<models::CapabilitiesResponse> {
        JsonResponse::Value(handler::capabilities())
    }

    /// Get the connector's schema.
    ///
    /// This function implements the [schema endpoint](https://hasura.github.io/ndc-spec/specification/schema/index.html)
    /// from the NDC specification.
    async fn get_schema(
        configuration: &Self::Configuration,
    ) -> Result<JsonResponse<models::SchemaResponse>, SchemaError> {
        handler::schema(configuration)
            .await
            .map(|config| JsonResponse::Value(config))
    }

    /// Explain a query by creating an execution plan
    ///
    /// This function implements the [explain endpoint](https://hasura.github.io/ndc-spec/specification/explain.html)
    /// from the NDC specification.
    async fn explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>, ExplainError> {
        handler::explain(configuration, state, request)
            .await
            .map(|explain| JsonResponse::Value(explain))
            .map_err(|err| ExplainError::Other(err.to_string().into()))
    }

    /// Execute a mutation
    ///
    /// This function implements the [mutation endpoint](https://hasura.github.io/ndc-spec/specification/mutations/index.html)
    /// from the NDC specification.
    async fn mutation(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::MutationResponse>, MutationError> {
        Err(MutationError::UnsupportedOperation(
            "mutation not supported".to_string(),
        ))
    }

    /// Execute a query
    ///
    /// This function implements the [query endpoint](https://hasura.github.io/ndc-spec/specification/queries/index.html)
    /// from the NDC specification.
    async fn query(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::QueryResponse>, QueryError> {
        handler::query(configuration, state, request)
            .await
            .map(|res| JsonResponse::Value(res))
            .map_err(|err| QueryError::Other(err.to_string().into()))
    }
}
