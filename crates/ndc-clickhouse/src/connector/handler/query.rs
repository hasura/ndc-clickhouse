use common::{client::execute_query, config::ServerConfig};
use ndc_sdk::{connector::QueryError, json_response::JsonResponse, models};
use tracing::{Instrument, Level};

use crate::{connector::state::ServerState, sql::QueryBuilder};

pub async fn query(
    configuration: &ServerConfig,
    state: &ServerState,
    request: models::QueryRequest,
) -> Result<JsonResponse<models::QueryResponse>, QueryError> {
    let (statement_string, parameters) =
        tracing::info_span!("Build SQL Query").in_scope(|| -> Result<_, QueryError> {
            let statement = QueryBuilder::new(&request, configuration)
                .build()
                .map_err(|err| QueryError::Other(Box::new(err)))?;

            let (statement, parameters) = statement.into_parameterized_statement();

            let statement_string = statement.to_parameterized_sql_string();

            Ok((statement_string, parameters))
        })?;

    let client = state
        .client(configuration)
        .await
        .map_err(|err| QueryError::Other(err.to_string().into()))?;

    let execution_span = tracing::info_span!("Execute SQL query", "query.SQL" = statement_string);

    let rowsets = execute_query(
        &client,
        &configuration.connection,
        &statement_string,
        &parameters,
    )
    .instrument(execution_span)
    .await
    .map_err(|err| QueryError::Other(err.to_string().into()))?;

    // we assume the response is a valid JSON string, and send those bytes back without parsing
    Ok(JsonResponse::Serialized(rowsets))
}
