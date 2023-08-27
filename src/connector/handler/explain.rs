use std::collections::BTreeMap;

use ndc_sdk::{connector::ExplainError, models};
use serde::{Deserialize, Serialize};

use crate::{
    connector::{client::execute_query, config::ServerConfig, state::ServerState},
    sql::QueryBuilder,
};

#[derive(Debug, Serialize, Deserialize)]
struct ExplainRow {
    explain: String,
}

pub async fn explain(
    configuration: &ServerConfig,
    state: &ServerState,
    request: models::QueryRequest,
) -> Result<models::ExplainResponse, ExplainError> {
    let unsafe_statement = QueryBuilder::new(&request, configuration)
        .build()
        .map_err(|err| ExplainError::Other(Box::new(err)))?;

    let unsafe_statement = unsafe_statement.explain();

    let (statement, parameters) = unsafe_statement.clone().into_parameterized_statement();

    let statement_string = statement.to_parameterized_sql_string();

    let client = state
        .client(configuration)
        .await
        .map_err(|err| ExplainError::Other(err))?;

    let explain = execute_query::<ExplainRow>(
        &client,
        &configuration.connection,
        &statement_string,
        &parameters,
    )
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|row| row.explain)
            .collect::<Vec<String>>()
            .join("\n")
    })
    .unwrap_or_else(|err| err.to_string());

    let details = BTreeMap::from_iter(vec![
        (
            "inlined_sql".to_string(),
            unsafe_statement.to_unsafe_sql_string(),
        ),
        ("parameterized_sql".to_string(), statement_string),
        (
            "parameters".to_string(),
            serde_json::to_string(&parameters).map_err(|err| ExplainError::Other(Box::new(err)))?,
        ),
        ("explain".to_string(), explain),
    ]);

    Ok(models::ExplainResponse { details })
}
