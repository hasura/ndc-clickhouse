use ndc_sdk::connector::UpdateConfigurationError;

use crate::connector::config::{get_server_config, ServerConfig};

pub async fn update_configuration(
    config: &ServerConfig,
) -> Result<ServerConfig, UpdateConfigurationError> {
    get_server_config(&config.connection)
        .await
        .map_err(|err| UpdateConfigurationError::Other(err))
}
