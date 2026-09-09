use common::{client::execute_bytes_query, config::ServerConfig};
use ndc_models as models;
use ndc_sdk_core::{connector::ErrorResponse, json_response::JsonResponse};
use tracing::Instrument;

use crate::{connector::state::ServerState, sql::QueryBuilder};

pub async fn query(
    configuration: &ServerConfig,
    state: &ServerState,
    request: models::QueryRequest,
) -> Result<JsonResponse<models::QueryResponse>, ErrorResponse> {
    #[cfg(debug_assertions)]
    {
        use tracing::Level;
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
                    use tracing::Level;
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

    let rowsets = execute_bytes_query(
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
        use tracing::Level;
        // this block only present in debug builds, to avoid leaking sensitive information
        let result_string = std::str::from_utf8(&rowsets).map_err(ErrorResponse::from_error)?;

        tracing::event!(Level::DEBUG, "Response" = result_string);
    }

    let rowsets = unwrap_json_compact_row(&rowsets).map_err(ErrorResponse::from_error)?;

    // the unwrapped value is the JSON document built by toJSONString; forward it without parsing
    Ok(JsonResponse::Serialized(rowsets.into()))
}

/// The query statement ends with `FORMAT JSONCompactEachRow`, so the response is a single line
/// `["<json>"]` holding the rowsets document that `toJSONString` produced. Unwrap that line to
/// the raw document bytes.
///
/// `TabSeparatedRaw` would hand back the document verbatim, but not every ClickHouse-compatible
/// HTTP endpoint accepts it (Tinybird's ClickHouse interface rejects it, for example), whereas
/// `JSONCompactEachRow` is universally available.
fn unwrap_json_compact_row(response: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let line = response
        .split(|b| *b == b'\n')
        .find(|line| !line.is_empty())
        .ok_or_else(|| std::io::Error::other("empty response from ClickHouse"))?;

    let mut row: Vec<String> = serde_json::from_slice(line).map_err(std::io::Error::other)?;

    match (row.pop(), row.is_empty()) {
        (Some(rowsets), true) => Ok(rowsets.into_bytes()),
        _ => Err(std::io::Error::other(
            "expected exactly one column in the ClickHouse response row",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::unwrap_json_compact_row;

    #[test]
    fn unwraps_the_rowsets_document() {
        let response = b"[\"[{\\\"rows\\\":[{\\\"a\\\":\\\"x\\\\ny\\\"}]}]\"]\n";
        let unwrapped = unwrap_json_compact_row(response).unwrap();
        assert_eq!(unwrapped, br#"[{"rows":[{"a":"x\ny"}]}]"#);
    }

    #[test]
    fn rejects_an_empty_response() {
        assert!(unwrap_json_compact_row(b"\n").is_err());
    }

    #[test]
    fn rejects_unexpected_column_counts() {
        assert!(unwrap_json_compact_row(b"[\"a\",\"b\"]\n").is_err());
        assert!(unwrap_json_compact_row(b"[]\n").is_err());
    }
}
