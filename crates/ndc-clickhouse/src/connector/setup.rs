use super::{state::ServerState, ClickhouseConnector};
use async_trait::async_trait;
use common::config::{read_server_config, ConfigurationEnvironment};
use ndc_sdk::connector::{self, Connector, ConnectorSetup, ErrorResponse};
use std::{collections::HashMap, path::Path};
#[derive(Debug, Clone)]
pub struct ClickhouseConnectorSetup(ConfigurationEnvironment);

#[async_trait]
impl ConnectorSetup for ClickhouseConnectorSetup {
    type Connector = ClickhouseConnector;

    async fn parse_configuration(
        &self,
        configuration_dir: impl AsRef<Path> + Send,
    ) -> connector::Result<<Self::Connector as Connector>::Configuration> {
        // we wrap read_server_config so the ParseError is implicitly converted into an ErrorResponse
        read_server_config(configuration_dir, &self.0)
            .await
            .map_err(ErrorResponse::from_error)
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
        Self(ConfigurationEnvironment::from_environment())
    }
}

impl ClickhouseConnectorSetup {
    pub fn new_from_env(env: HashMap<String, String>) -> Self {
        Self(ConfigurationEnvironment::from_simulated_environment(env))
    }
}
