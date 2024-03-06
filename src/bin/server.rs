use ndc_clickhouse::connector::ClickhouseConnector;
use ndc_sdk::default_main::default_main;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    default_main::<ClickhouseConnector>().await
}
