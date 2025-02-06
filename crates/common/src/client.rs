use crate::config::ConnectionConfig;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::error::Error;
use tracing::Instrument;

pub fn get_http_client(
    _connection_config: &ConnectionConfig,
) -> Result<reqwest::Client, reqwest::Error> {
    // todo: we could make client come preconfigured with some headers such as for username and password?
    let client = reqwest::Client::builder().build()?;
    Ok(client)
}

async fn execute_query(
    client: &reqwest::Client,
    connection_config: &ConnectionConfig,
    statement: &str,
    parameters: &Vec<(String, String)>,
) -> Result<reqwest::Response, reqwest::Error> {
    client
        .post(&connection_config.url)
        .header("X-ClickHouse-User", &connection_config.username)
        .header("X-ClickHouse-Key", &connection_config.password)
        .query(parameters)
        .body(statement.to_owned())
        .send()
        .instrument(tracing::info_span!(
            "Execute HTTP request",
            internal.visibility = "user"
        ))
        .await
}

pub async fn execute_bytes_query(
    client: &reqwest::Client,
    connection_config: &ConnectionConfig,
    statement: &str,
    parameters: &Vec<(String, String)>,
) -> Result<Bytes, reqwest::Error> {
    let response = execute_query(client, connection_config, statement, parameters).await?;

    let response = response
        .error_for_status()?
        .bytes()
        .instrument(tracing::info_span!(
            "Read HTTP response",
            internal.visibility = "user"
        ))
        .await?;

    Ok(response)
}

pub async fn execute_text_query<T: DeserializeOwned>(
    client: &reqwest::Client,
    connection_config: &ConnectionConfig,
    statement: &str,
    parameters: &Vec<(String, String)>,
) -> Result<String, reqwest::Error> {
    let response = execute_query(client, connection_config, statement, parameters).await?;

    let response = response
        .error_for_status()?
        .text()
        .instrument(tracing::info_span!("Parse HTTP response"))
        .await?;

    Ok(response)
}

pub async fn execute_json_query<T: DeserializeOwned>(
    client: &reqwest::Client,
    connection_config: &ConnectionConfig,
    statement: &str,
    parameters: &Vec<(String, String)>,
) -> Result<T, reqwest::Error> {
    let response = execute_query(client, connection_config, statement, parameters).await?;

    let response: T = response
        .error_for_status()?
        .json()
        .instrument(tracing::info_span!("Parse HTTP response"))
        .await?;

    Ok(response)
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
