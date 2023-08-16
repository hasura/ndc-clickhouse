mod config;

use async_trait::async_trait;
use config::ServerConfig;
use ndc_sdk::{
    connector::{
        Connector, ExplainError, FetchMetricsError, HealthError, InitializationError,
        MutationError, QueryError, SchemaError, UpdateConfigurationError, ValidateError,
    },
    default_main::default_main,
    models,
};
use std::{error::Error, sync::Arc};

#[derive(Debug, Clone, clap::Args)]
struct ServerArgs {}

#[derive(Debug)]
struct ServerState {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    default_main::<Clickhouse>().await?;

    Ok(())
}

#[derive(Debug, Clone, Default)]
struct Clickhouse;

#[async_trait]
impl Connector for Clickhouse {
    /// The type of unvalidated, raw configuration, as provided by the user.
    type RawConfiguration = ServerConfig;

    /// The type of validated configuration
    type Configuration = ServerConfig;

    /// The type of unserializable state
    type State = Arc<ServerState>;

    fn make_empty_configuration() -> Self::RawConfiguration {
        ServerConfig::default()
    }

    async fn update_configuration(
        config: &Self::RawConfiguration,
    ) -> Result<Self::RawConfiguration, UpdateConfigurationError> {
        Ok(config.to_owned())
    }

    /// Validate the raw configuration provided by the user,
    /// returning a configuration error or a validated [`Connector::Configuration`].
    async fn validate_raw_configuration(
        configuration: &Self::RawConfiguration,
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
        metrics: &mut prometheus::Registry,
    ) -> Result<Self::State, InitializationError> {
        Ok(Arc::new(ServerState {}))
    }

    /// Update any metrics from the state
    ///
    /// Note: some metrics can be updated directly, and do not
    /// need to be updated here. This function can be useful to
    /// query metrics which cannot be updated directly, e.g.
    /// the number of idle connections in a connection pool
    /// can be polled but not updated directly.
    fn fetch_metrics(
        configuration: &Self::Configuration,
        state: &Self::State,
    ) -> Result<(), FetchMetricsError> {
        todo!()
    }

    /// Check the health of the connector.
    ///
    /// For example, this function should check that the connector
    /// is able to reach its data source over the network.
    async fn health_check(
        configuration: &Self::Configuration,
        state: &Self::State,
    ) -> Result<(), HealthError> {
        todo!()
    }

    /// Get the connector's capabilities.
    ///
    /// This function implements the [capabilities endpoint](https://hasura.github.io/ndc-spec/specification/capabilities.html)
    /// from the NDC specification.
    async fn get_capabilities() -> models::CapabilitiesResponse {
        todo!()
    }

    /// Get the connector's schema.
    ///
    /// This function implements the [schema endpoint](https://hasura.github.io/ndc-spec/specification/schema/index.html)
    /// from the NDC specification.
    async fn get_schema(
        configuration: &Self::Configuration,
    ) -> Result<models::SchemaResponse, SchemaError> {
        todo!()
    }

    /// Explain a query by creating an execution plan
    ///
    /// This function implements the [explain endpoint](https://hasura.github.io/ndc-spec/specification/explain.html)
    /// from the NDC specification.
    async fn explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<models::ExplainResponse, ExplainError> {
        todo!()
    }

    /// Execute a mutation
    ///
    /// This function implements the [mutation endpoint](https://hasura.github.io/ndc-spec/specification/mutations/index.html)
    /// from the NDC specification.
    async fn mutation(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::MutationRequest,
    ) -> Result<models::MutationResponse, MutationError> {
        todo!()
    }

    /// Execute a query
    ///
    /// This function implements the [query endpoint](https://hasura.github.io/ndc-spec/specification/queries/index.html)
    /// from the NDC specification.
    async fn query(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<models::QueryResponse, QueryError> {
        todo!()
    }
}
