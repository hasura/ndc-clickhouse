use common::clickhouse_parser::parameterized_query::ParameterType;
use ndc_sdk::connector::{ExplainError, QueryError};
use std::fmt;

use super::typecasting::TypeStringError;

#[derive(Debug, PartialEq)]
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
    /// A field was referenced but not found in configuration
    UnknownSubField {
        field_name: String,
        data_type: String,
    },
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
    /// Column type did not match type asserted by request
    ColumnTypeMismatch { expected: String, got: String },
    /// Attempted to cast a JSON value to a mismatching data type
    UnsupportedParameterCast {
        value: serde_json::Value,
        data_type: ParameterType,
    },
    /// Attempted to cast a value to a tuple with a mismatching length
    TupleLengthMismatch {
        value: serde_json::Value,
        data_type: ParameterType,
    },
    /// Attempted to cast an Array to a named tuple, which should be represented as an object
    ExpectedAnonymousTuple {
        value: serde_json::Value,
        data_type: ParameterType,
    },
    /// Attempted to cast an Object to an anonymous tuple, which should be represented as an array
    ExpectedNamedTuple {
        value: serde_json::Value,
        data_type: ParameterType,
    },
    /// could not find field required by named tuple or nested in the source json object
    MissingNamedField {
        value: serde_json::Value,
        data_type: ParameterType,
        field: String,
    },
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
            QueryBuilderError::UnknownSubField {
                field_name,
                data_type,
            } => {
                write!(f, "Unknown field {field_name} in type {data_type}")
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
            QueryBuilderError::ColumnTypeMismatch { expected, got } => {
                write!(f, "Column Type Mismatch: expected {expected}, got {got}")
            }
            QueryBuilderError::UnsupportedParameterCast { value, data_type } => {
                write!(f, "Cannot cast value `{}` to type `{}`", value, data_type)
            }
            QueryBuilderError::TupleLengthMismatch { value, data_type } => {
                write!(f, "Tuple `{data_type}` length does not match value {value}")
            }
            QueryBuilderError::ExpectedAnonymousTuple { value, data_type } => {
                write!(
                    f,
                    "Expected anonymous tuple for value `{value}`, got `{data_type}`"
                )
            }
            QueryBuilderError::ExpectedNamedTuple { value, data_type } => write!(
                f,
                "Expected named tuple for value `{value}`, got `{data_type}`"
            ),
            QueryBuilderError::MissingNamedField {
                value,
                data_type,
                field,
            } => write!(f, "Missing field `{field}` for `{data_type}` in `{value}`"),
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
            | QueryBuilderError::UnknownSubField { .. }
            | QueryBuilderError::CannotSerializeVariables(_)
            | QueryBuilderError::UnknownSingleColumnAggregateFunction(_)
            | QueryBuilderError::UnknownBinaryComparisonOperator(_)
            | QueryBuilderError::Typecasting(_)
            | QueryBuilderError::ColumnTypeMismatch { .. }
            | QueryBuilderError::UnsupportedParameterCast { .. }
            | QueryBuilderError::ExpectedAnonymousTuple { .. }
            | QueryBuilderError::ExpectedNamedTuple { .. }
            | QueryBuilderError::MissingNamedField { .. }
            | QueryBuilderError::TupleLengthMismatch { .. } => {
                QueryError::new_invalid_request(&value)
            }
            QueryBuilderError::NotSupported(_) => QueryError::new_unsupported_operation(&value),
            QueryBuilderError::Unexpected(_) => QueryError::new(value),
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
            | QueryBuilderError::UnknownSubField { .. }
            | QueryBuilderError::CannotSerializeVariables(_)
            | QueryBuilderError::UnknownSingleColumnAggregateFunction(_)
            | QueryBuilderError::UnknownBinaryComparisonOperator(_)
            | QueryBuilderError::Typecasting(_)
            | QueryBuilderError::ColumnTypeMismatch { .. }
            | QueryBuilderError::UnsupportedParameterCast { .. }
            | QueryBuilderError::ExpectedAnonymousTuple { .. }
            | QueryBuilderError::ExpectedNamedTuple { .. }
            | QueryBuilderError::MissingNamedField { .. }
            | QueryBuilderError::TupleLengthMismatch { .. } => {
                ExplainError::new_invalid_request(&value)
            }
            QueryBuilderError::NotSupported(_) => ExplainError::new_unsupported_operation(&value),
            QueryBuilderError::Unexpected(_) => ExplainError::new(Box::new(value)),
        }
    }
}
