use super::typecasting::TypeStringError;
use common::clickhouse_parser::{datatype::ClickHouseDataType, parameterized_query::ParameterType};
use http::StatusCode;
use ndc_models::{
    AggregateFunctionName, ArgumentName, CollectionName, ComparisonOperatorName, FieldName,
    ObjectTypeName, RelationshipName,
};
use ndc_sdk_core::connector::ErrorResponse;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum QueryBuilderError {
    /// A relationship referenced in the query is missing from the collection_relationships map
    #[error("Missing relationship: {0}")]
    MissingRelationship(RelationshipName),
    /// An argument required for a native query was not supplied
    #[error("Argument {argument} required for native query {query} was not supplied")]
    MissingNativeQueryArgument {
        query: CollectionName,
        argument: ArgumentName,
    },
    /// A table was referenced but not found in configuration
    #[error("Unable to find table {0} in config")]
    UnknownTable(CollectionName),
    /// An argument was supplied for a table that does not have that argument
    #[error("Unknown argument {argument} supplied for table {table}")]
    UnknownTableArgument {
        table: CollectionName,
        argument: ArgumentName,
    },
    /// An argument was supplied for a query that does not have that argument
    #[error("Unknown argument {argument} supplied for query {query}")]
    UnknownQueryArgument {
        query: CollectionName,
        argument: ArgumentName,
    },
    /// A table in configuration referenced a table type that could not be found
    #[error("Unable to find table type {0} in config")]
    UnknownTableType(ObjectTypeName),
    /// A column was referenced but not found in configuration
    #[error("Unable to find column {0} for table {1} in config")]
    UnknownColumn(FieldName, ObjectTypeName),
    /// A field was referenced but not found in configuration
    #[error("Unknown field {field_name} in type {data_type}")]
    UnknownSubField {
        field_name: FieldName,
        data_type: ClickHouseDataType,
    },
    /// Unable to serialize variables into a json string
    #[error("Unable to serialize variables into a json string: {0}")]
    CannotSerializeVariables(String),
    /// An unknown single column aggregate function was referenced
    #[error("Unknown single column aggregate function: {0}")]
    UnknownSingleColumnAggregateFunction(AggregateFunctionName),
    /// An unknown binary comparison operator was referenced
    #[error("Unknown binary comparison operator: {0}")]
    UnknownBinaryComparisonOperator(ComparisonOperatorName),
    /// A feature is not supported
    #[error("Not supported: {0}")]
    NotSupported(String),
    /// An error that should never happen, and indicates a bug if triggered
    #[error("Unexpected: {0}")]
    Unexpected(String),
    /// There was an issue creating typecasting strings
    #[error("Typecasting: {0}")]
    Typecasting(TypeStringError),
    /// Column type did not match type asserted by request
    #[error("Column Type Mismatch: expected {expected}, got {got}")]
    ColumnTypeMismatch { expected: String, got: String },
    /// Attempted to cast a JSON value to a mismatching data type
    #[error("Cannot cast value `{value}` to type `{data_type}`")]
    UnsupportedParameterCast {
        value: serde_json::Value,
        data_type: ParameterType,
    },
    /// Attempted to cast a value to a tuple with a mismatching length
    #[error("Tuple `{data_type}` length does not match value {value}")]
    TupleLengthMismatch {
        value: serde_json::Value,
        data_type: ParameterType,
    },
    /// Attempted to cast an Array to a named tuple, which should be represented as an object
    #[error("Expected anonymous tuple for value `{value}`, got `{data_type}`")]
    ExpectedAnonymousTuple {
        value: serde_json::Value,
        data_type: ParameterType,
    },
    /// Attempted to cast an Object to an anonymous tuple, which should be represented as an array
    #[error("Expected named tuple for value `{value}`, got `{data_type}`")]
    ExpectedNamedTuple {
        value: serde_json::Value,
        data_type: ParameterType,
    },
    /// could not find field required by named tuple or nested in the source json object
    #[error("Missing field `{field}` for `{data_type}` in `{value}`")]
    MissingNamedField {
        value: serde_json::Value,
        data_type: ParameterType,
        field: String,
    },
}

impl From<QueryBuilderError> for ErrorResponse {
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
            | QueryBuilderError::TupleLengthMismatch { .. } => ErrorResponse::new(
                StatusCode::BAD_REQUEST,
                value.to_string(),
                serde_json::Value::Null,
            ),
            QueryBuilderError::NotSupported(_) => ErrorResponse::new(
                StatusCode::NOT_IMPLEMENTED,
                value.to_string(),
                serde_json::Value::Null,
            ),
            QueryBuilderError::Unexpected(_) => ErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                value.to_string(),
                serde_json::Value::Null,
            ),
        }
    }
}
