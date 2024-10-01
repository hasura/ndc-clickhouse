use common::{client::execute_query, config::ServerConfig};
use ndc_sdk::{connector::ErrorResponse, json_response::JsonResponse, models};
use tracing::{Instrument, Level};

use crate::{connector::state::ServerState, sql::QueryBuilder};

pub async fn query(
    configuration: &ServerConfig,
    state: &ServerState,
    request: models::QueryRequest,
) -> Result<JsonResponse<models::QueryResponse>, ErrorResponse> {
    #[cfg(debug_assertions)]
    {
        // this block only present in debug builds, to avoid leaking sensitive information
        let request_string = serde_json::to_string(&request).map_err(ErrorResponse::from_error)?;

        tracing::event!(Level::DEBUG, "Incoming IR" = request_string);
    }

    let (statement_string, parameters) =
        tracing::info_span!("Build SQL Query", internal.visibility = "user").in_scope(
            || -> Result<_, ErrorResponse> {
                let (statement, parameters) =
                    QueryBuilder::new(&request, configuration).build_parameterized()?;

                #[cfg(debug_assertions)]
                {
                    // this block only present in debug builds, to avoid leaking sensitive information
                    let unsafe_statement_string = QueryBuilder::new(&request, configuration)
                        .build_inlined()?
                        .to_string();

                    tracing::event!(Level::DEBUG, "Generated SQL" = unsafe_statement_string);
                }

                let statement_string = statement.to_string();

                Ok((statement_string, parameters))
            },
        )?;

    let client = state
        .client(configuration)
        .await
        .map_err(ErrorResponse::from_error)?;

    let execution_span = tracing::info_span!(
        "Execute SQL query",
        db.system = "clickhouse",
        db.user = configuration.connection.username,
        db.statement = statement_string,
        internal.visibility = "user",
    );

    let rowsets = execute_query(
        &client,
        &configuration.connection,
        &statement_string,
        &parameters,
    )
    .instrument(execution_span)
    .await
    .map_err(ErrorResponse::from_error)?;

    #[cfg(debug_assertions)]
    {
        // this block only present in debug builds, to avoid leaking sensitive information
        let result_string = std::str::from_utf8(&rowsets).map_err(ErrorResponse::from_error)?;

        tracing::event!(Level::DEBUG, "Response" = result_string);
    }

    // we assume the response is a valid JSON string, and send those bytes back without parsing
    Ok(JsonResponse::Serialized(rowsets))
}
