use std::error::Error;

use config::ConnectionConfig;
use serde::{de::DeserializeOwned, Deserialize};

pub fn get_http_client(
    _connection_config: &ConnectionConfig,
) -> Result<reqwest::Client, Box<dyn Error>> {
    // todo: we could make client come preconfigured with some headers such as for username and password?
    let client = reqwest::Client::builder().build()?;
    Ok(client)
}

pub async fn execute_query<T: DeserializeOwned>(
    client: &reqwest::Client,
    connection_config: &ConnectionConfig,
    statement: &str,
    parameters: &Vec<(String, String)>,
) -> Result<Vec<T>, Box<dyn Error>> {
    let response = client
        .post(&connection_config.url)
        .header("X-ClickHouse-User", &connection_config.username)
        .header("X-ClickHouse-Key", &connection_config.password)
        .query(parameters)
        .body(statement.to_owned())
        .send()
        .await?;

    if response.error_for_status_ref().is_err() {
        return Err(response.text().await?.into());
    }

    let payload: ClickHouseResponse<T> = response.json().await?;

    Ok(payload.data)
}

pub async fn ping(
    client: &reqwest::Client,
    connection_config: &ConnectionConfig,
) -> Result<(), Box<dyn Error>> {
    let last_char = connection_config.url.chars().last();

    let url = if let Some('/') = last_char {
        format!("{}ping", connection_config.url)
    } else {
        format!("{}/ping", connection_config.url)
    };

    let _request = client
        .get(&url)
        .header("X-ClickHouse-User", &connection_config.username)
        .header("X-ClickHouse-Key", &connection_config.password)
        .send()
        .await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ClickHouseResponse<T> {
    #[allow(dead_code)]
    meta: Vec<ClickHouseResponseMeta>,
    data: Vec<T>,
    #[allow(dead_code)]
    rows: u32,
    // unsure about the specification for this object, it's likely to be somewhat dynamic
    // keeping as an unspecified json value for now
    #[allow(dead_code)]
    statistics: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ClickHouseResponseMeta {
    name: String,
    #[serde(rename = "type")]
    column_type: String,
}
