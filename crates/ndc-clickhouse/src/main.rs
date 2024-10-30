use ndc_clickhouse_core::connector::setup::ClickhouseConnectorSetup;
use ndc_sdk::{connector::ErrorResponse, default_main::default_main};

#[tokio::main]
async fn main() -> Result<(), ErrorResponse> {
    default_main::<ClickhouseConnectorSetup>().await
}
