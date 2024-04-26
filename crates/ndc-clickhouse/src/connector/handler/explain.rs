use std::collections::BTreeMap;

use common::{client::execute_json_query, config::ServerConfig};
use ndc_sdk::{connector::ExplainError, json_response::JsonResponse, models};
use serde::{Deserialize, Serialize};

use crate::{connector::state::ServerState, sql::QueryBuilder};

#[derive(Debug, Serialize, Deserialize)]
struct ExplainRow {
    explain: String,
}

pub async fn explain(
    configuration: &ServerConfig,
    state: &ServerState,
    request: models::QueryRequest,
) -> Result<JsonResponse<models::ExplainResponse>, ExplainError> {
    let unsafe_statement = QueryBuilder::new(&request, configuration).build()?;

    let unsafe_statement = unsafe_statement.explain();

    let (statement, parameters) = unsafe_statement.clone().into_parameterized_statement();

    let statement_string = statement.to_parameterized_sql_string();

    let client = state
        .client(configuration)
        .await
        .map_err(|err| ExplainError::Other(err.to_string().into()))?;

    let explain = execute_json_query::<ExplainRow>(
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
            "SQL Query".to_string(),
            add_variables_note(
                &request,
                &pretty_print_sql(&unsafe_statement.to_unsafe_sql_string()),
            ),
        ),
        (
            "Parameterized SQL Query".to_string(),
            add_variables_note(&request, &pretty_print_sql(&statement_string)),
        ),
        (
            "Parameters".to_string(),
            serde_json::to_string(&parameters).map_err(|err| ExplainError::Other(Box::new(err)))?,
        ),
        ("Execution Plan".to_string(), explain),
    ]);

    Ok(JsonResponse::Value(models::ExplainResponse { details }))
}

fn pretty_print_sql(query: &str) -> String {
    use sqlformat::{format, FormatOptions, Indent, QueryParams};
    let params = QueryParams::None;
    let options = FormatOptions {
        indent: Indent::Spaces(2),
        uppercase: false,
        lines_between_queries: 1,
    };

    format(query, &params, options)
}

const EXPLAIN_NOTE: &str = r#"-- note: the source object for _vars should be a JSON string of the form
-- `{"_varset_id": [1,2,3], "_var_ID": [1,2,3], "_var_NAME": ["Name1","Name2","Name3"]}`
-- The example assumes the variables ID and NAME, change as appropriate. "_varset_id" is an index starting from 1
-- Each array member corresponds to a row, all arrays should have the same number of members. See clickhouse docs for more:
-- https://clickhouse.com/docs/en/interfaces/formats#jsoncolumns
-- https://clickhouse.com/docs/en/sql-reference/table-functions/format
"#;

fn add_variables_note(request: &models::QueryRequest, statement: &str) -> String {
    if request.variables.is_some() {
        format!("{EXPLAIN_NOTE}\n{statement}")
    } else {
        statement.to_owned()
    }
}
