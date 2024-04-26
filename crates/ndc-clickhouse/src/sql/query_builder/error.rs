use std::fmt;

use ndc_sdk::connector::{ExplainError, QueryError};

use super::typecasting::TypeStringError;

#[derive(Debug)]
pub enum QueryBuilderError {
    /// A relationship referenced in the query is missing from the collection_relationships map
    MissingRelationship(String),
    /// An argument required for a native query was not supplied
    MissingNativeQueryArgument { query: String, argument: String },
    /// A table was referenced but not found in configuration
    UnknownTable(String),
    /// An argument was supplied for a table that does not have that argument
    UnknownTableArgument { table: String, argument: String },
    /// An argument was supplied for a table that does not have that argument
    UnknownQueryArgument { query: String, argument: String },
    /// A table in configuration referenced a table type that could not be found
    UnknownTableType(String),
    /// A column was referenced but not found in configuration
    UnknownColumn(String, String),
    /// Unable to serialize variables into a json string
    CannotSerializeVariables(String),
    /// An unknown single column aggregate function was referenced
    UnknownSingleColumnAggregateFunction(String),
    /// An unknown binary comparison operator was referenced
    UnknownBinaryComparisonOperator(String),
    /// A feature is not supported
    NotSupported(String),
    /// An error that should never happen, and indicates a bug if triggered
    Unexpected(String),
    /// There was an issue creating typecasting strings
    Typecasting(TypeStringError),
}

impl fmt::Display for QueryBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryBuilderError::MissingRelationship(rel) => write!(f, "Missing relationship: {rel}"),
            QueryBuilderError::MissingNativeQueryArgument { query, argument } => write!(
                f,
                "Argument {argument} required for native query {query} was not supplied"
            ),
            QueryBuilderError::UnknownTable(t) => write!(f, "Unable to find table {t} in config"),
            QueryBuilderError::UnknownTableArgument { table, argument } => {
                write!(f, "Unknown argument {argument} supplied for table {table}")
            }
            QueryBuilderError::UnknownQueryArgument { query, argument } => {
                write!(f, "Unknown argument {argument} supplied for query {query}")
            }

            QueryBuilderError::UnknownTableType(t) => {
                write!(f, "Unable to find table type {t} in config")
            }
            QueryBuilderError::UnknownColumn(c, t) => {
                write!(f, "Unable to find column {c} for table {t} in config")
            }
            QueryBuilderError::CannotSerializeVariables(e) => {
                write!(f, "Unable to serialize variables into a json string: {e}")
            }
            QueryBuilderError::UnknownSingleColumnAggregateFunction(agg) => {
                write!(f, "Unknown single column aggregate function: {agg}")
            }
            QueryBuilderError::UnknownBinaryComparisonOperator(op) => {
                write!(f, "Unknown binary comparison operator: {op}")
            }
            QueryBuilderError::NotSupported(e) => write!(f, "Not supported: {e}"),
            QueryBuilderError::Unexpected(e) => write!(f, "Unexpected: {e}"),
            QueryBuilderError::Typecasting(e) => write!(f, "Typecasting: {e}"),
        }
    }
}

impl std::error::Error for QueryBuilderError {}

impl From<QueryBuilderError> for QueryError {
    fn from(value: QueryBuilderError) -> Self {
        match value {
            QueryBuilderError::MissingRelationship(_)
            | QueryBuilderError::MissingNativeQueryArgument { .. }
            | QueryBuilderError::UnknownTable(_)
            | QueryBuilderError::UnknownTableArgument { .. }
            | QueryBuilderError::UnknownQueryArgument { .. }
            | QueryBuilderError::UnknownTableType(_)
            | QueryBuilderError::UnknownColumn(_, _)
            | QueryBuilderError::CannotSerializeVariables(_)
            | QueryBuilderError::UnknownSingleColumnAggregateFunction(_)
            | QueryBuilderError::UnknownBinaryComparisonOperator(_)
            | QueryBuilderError::Typecasting(_) => {
                QueryError::UnprocessableContent(value.to_string())
            }
            QueryBuilderError::NotSupported(_) => {
                QueryError::UnsupportedOperation(value.to_string())
            }
            QueryBuilderError::Unexpected(_) => QueryError::Other(Box::new(value)),
        }
    }
}

impl From<QueryBuilderError> for ExplainError {
    fn from(value: QueryBuilderError) -> Self {
        match value {
            QueryBuilderError::MissingRelationship(_)
            | QueryBuilderError::MissingNativeQueryArgument { .. }
            | QueryBuilderError::UnknownTable(_)
            | QueryBuilderError::UnknownTableArgument { .. }
            | QueryBuilderError::UnknownQueryArgument { .. }
            | QueryBuilderError::UnknownTableType(_)
            | QueryBuilderError::UnknownColumn(_, _)
            | QueryBuilderError::CannotSerializeVariables(_)
            | QueryBuilderError::UnknownSingleColumnAggregateFunction(_)
            | QueryBuilderError::UnknownBinaryComparisonOperator(_)
            | QueryBuilderError::Typecasting(_) => {
                ExplainError::UnprocessableContent(value.to_string())
            }
            QueryBuilderError::NotSupported(_) => {
                ExplainError::UnsupportedOperation(value.to_string())
            }
            QueryBuilderError::Unexpected(_) => ExplainError::Other(Box::new(value)),
        }
    }
}
