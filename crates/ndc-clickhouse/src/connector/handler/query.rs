use common::{client::execute_query, config::ServerConfig};
use ndc_sdk::{connector::QueryError, models};

use crate::{connector::state::ServerState, sql::QueryBuilder};

pub async fn query(
    configuration: &ServerConfig,
    state: &ServerState,
    request: models::QueryRequest,
) -> Result<models::QueryResponse, QueryError> {
    let statement = QueryBuilder::new(&request, configuration)
        .build()
        .map_err(|err| QueryError::Other(Box::new(err)))?;

    let (statement, parameters) = statement.into_parameterized_statement();

    let statement_string = statement.to_parameterized_sql_string();

    let client = state
        .client(configuration)
        .await
        .map_err(|err| QueryError::Other(err.to_string().into()))?;

    let rowsets = execute_query::<models::RowSet>(
        &client,
        &configuration.connection,
        &statement_string,
        &parameters,
    )
    .await
    .map_err(|err| QueryError::Other(err.to_string().into()))?;

    Ok(models::QueryResponse(rowsets))
}
