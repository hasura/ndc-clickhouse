use std::{collections::BTreeMap, fmt::Display, str::FromStr};

use indexmap::IndexMap;
use ndc_sdk::models;

use crate::{
    connector::config::{ColumnConfig, ServerConfig},
    schema::{ClickHouseScalarType, ClickhouseDataType},
};

/// Tuple(rows <RowsCastString>, aggregates <RowsCastString>)
pub struct RowsetTypeString {
    rows: Option<RowsTypeString>,
    aggregates: Option<AggregatesTypeString>,
}
/// Tuple("a1" T1, "a2" T2)
pub struct AggregatesTypeString {
    aggregates: Vec<(String, String)>,
}
/// Tuple("f1" T1, "f2" <RowSetTypeString>)
pub struct RowsTypeString {
    fields: Vec<(String, FieldTypeString)>,
}
pub enum FieldTypeString {
    Relationship(RowsetTypeString),
    Column(String),
}

impl RowsetTypeString {
    pub fn new(
        table_alias: &str,
        query: &models::Query,
        relationships: &BTreeMap<String, models::Relationship>,
        config: &ServerConfig,
    ) -> Result<Self, TypeStringError> {
        let rows = if let Some(fields) = &query.fields {
            Some(RowsTypeString::new(
                table_alias,
                fields,
                relationships,
                config,
            )?)
        } else {
            None
        };
        let aggregates = if let Some(aggregates) = &query.aggregates {
            Some(AggregatesTypeString::new(table_alias, aggregates, config)?)
        } else {
            None
        };

        Ok(Self { rows, aggregates })
    }
}

impl AggregatesTypeString {
    pub fn new(
        table_alias: &str,
        aggregates: &IndexMap<String, models::Aggregate>,
        config: &ServerConfig,
    ) -> Result<Self, TypeStringError> {
        Ok(Self {
            aggregates: aggregates
                .iter()
                .map(|(alias, aggregate)| match aggregate {
                    models::Aggregate::StarCount {} | models::Aggregate::ColumnCount { .. } => {
                        Ok((alias.to_string(), "UInt32".to_string()))
                    }
                    models::Aggregate::SingleColumn {
                        column: column_alias,
                        function,
                    } => {
                        let column = get_column(column_alias, table_alias, config)?;
                        let scalar_type =
                            get_scalar_column_type(column, column_alias, table_alias)?;

                        let aggregate_functions = scalar_type.aggregate_functions();

                        let result_type = aggregate_functions
                            .iter()
                            .find(|(f, _)| &f.to_string() == function)
                            .map(|(_, r)| r)
                            .ok_or_else(|| TypeStringError::UnknownAggregateFunction {
                                table: table_alias.to_owned(),
                                column: column_alias.to_owned(),
                                data_type: column.data_type.to_owned(),
                                scalar_type: scalar_type.to_string(),
                                function: function.to_owned(),
                            })?;

                        Ok((alias.to_string(), result_type.to_string()))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl RowsTypeString {
    pub fn new(
        table_alias: &str,
        fields: &IndexMap<String, models::Field>,
        relationships: &BTreeMap<String, models::Relationship>,
        config: &ServerConfig,
    ) -> Result<Self, TypeStringError> {
        Ok(Self {
            fields: fields
                .iter()
                .map(|(alias, field)| {
                    Ok((
                        alias.to_string(),
                        match field {
                            models::Field::Column {
                                column: column_alias,
                            } => {
                                let column = get_column(column_alias, table_alias, config)?;
                                FieldTypeString::Column(column.data_type.to_owned())
                            }
                            models::Field::Relationship {
                                query,
                                relationship,
                                arguments: _,
                            } => {
                                let relationship =
                                    relationships.get(relationship).ok_or_else(|| {
                                        TypeStringError::MissingRelationship(
                                            relationship.to_owned(),
                                        )
                                    })?;

                                let table_alias = &relationship.target_collection;

                                FieldTypeString::Relationship(RowsetTypeString::new(
                                    table_alias,
                                    query,
                                    relationships,
                                    config,
                                )?)
                            }
                        },
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl Display for RowsetTypeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.rows, &self.aggregates) {
            (None, None) => write!(f, "Map(Nothing, Nothing)"),
            (None, Some(aggregates)) => write!(f, "Tuple(aggregates {aggregates})"),
            (Some(rows), None) => write!(f, "Tuple(rows {rows})"),
            (Some(rows), Some(aggregates)) => {
                write!(f, "Tuple(rows {rows}, aggregates {aggregates})")
            }
        }
    }
}
impl Display for AggregatesTypeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.aggregates.is_empty() {
            write!(f, "Map(Nothing, Nothing)")
        } else {
            write!(f, "Tuple(")?;
            let mut first = true;

            for (alias, t) in &self.aggregates {
                if first {
                    first = false;
                } else {
                    write!(f, ", ")?;
                }

                write!(f, "\"{alias}\" {t}")?;
            }

            write!(f, ")")
        }
    }
}
impl Display for RowsTypeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.fields.is_empty() {
            write!(f, "Array(Map(Nothing, Nothing))")
        } else {
            write!(f, "Array(Tuple(")?;
            let mut first = true;

            for (alias, field) in &self.fields {
                if first {
                    first = false;
                } else {
                    write!(f, ", ")?;
                }

                write!(f, "\"{alias}\" ")?;

                match field {
                    FieldTypeString::Column(c) => {
                        write!(f, "{c}")?;
                    }
                    FieldTypeString::Relationship(r) => {
                        write!(f, "{r}")?;
                    }
                }
            }

            write!(f, "))")
        }
    }
}

fn get_column<'a>(
    column_alias: &str,
    table_alias: &str,
    config: &'a ServerConfig,
) -> Result<&'a ColumnConfig, TypeStringError> {
    let table = config
        .tables
        .iter()
        .find(|t| t.alias == table_alias)
        .ok_or_else(|| TypeStringError::UnknownTable {
            table: table_alias.to_owned(),
        })?;

    let column = table
        .columns
        .iter()
        .find(|c| c.alias == column_alias)
        .ok_or_else(|| TypeStringError::UnknownColumn {
            table: table_alias.to_owned(),
            column: column_alias.to_owned(),
        })?;

    Ok(column)
}

fn get_scalar_column_type(
    column: &ColumnConfig,
    column_alias: &str,
    table_alias: &str,
) -> Result<ClickHouseScalarType, TypeStringError> {
    let data_type = ClickhouseDataType::from_str(&column.data_type).map_err(|_err| {
        TypeStringError::CannotParseTypeString {
            table: table_alias.to_owned(),
            column: column_alias.to_owned(),
            data_type: column.data_type.to_owned(),
        }
    })?;

    let scalar_type =
        get_scalar_type(&data_type).ok_or_else(|| TypeStringError::ColumnNotScalar {
            table: table_alias.to_owned(),
            column: column_alias.to_owned(),
            data_type: column.data_type.to_owned(),
        })?;

    Ok(scalar_type)
}

fn get_scalar_type(data_type: &ClickhouseDataType) -> Option<ClickHouseScalarType> {
    match data_type {
        ClickhouseDataType::Nullable(data_type) => get_scalar_type(data_type),
        ClickhouseDataType::Bool => Some(ClickHouseScalarType::Bool),
        ClickhouseDataType::String | ClickhouseDataType::FixedString(_) => {
            Some(ClickHouseScalarType::String)
        }
        ClickhouseDataType::UInt8 => Some(ClickHouseScalarType::UInt8),
        ClickhouseDataType::UInt16 => Some(ClickHouseScalarType::UInt16),
        ClickhouseDataType::UInt32 => Some(ClickHouseScalarType::UInt32),
        ClickhouseDataType::UInt64 => Some(ClickHouseScalarType::UInt64),
        ClickhouseDataType::UInt128 => Some(ClickHouseScalarType::UInt128),
        ClickhouseDataType::UInt256 => Some(ClickHouseScalarType::UInt256),
        ClickhouseDataType::Int8 => Some(ClickHouseScalarType::Int8),
        ClickhouseDataType::Int16 => Some(ClickHouseScalarType::Int16),
        ClickhouseDataType::Int32 => Some(ClickHouseScalarType::Int32),
        ClickhouseDataType::Int64 => Some(ClickHouseScalarType::Int64),
        ClickhouseDataType::Int128 => Some(ClickHouseScalarType::Int128),
        ClickhouseDataType::Int256 => Some(ClickHouseScalarType::Int256),
        ClickhouseDataType::Float32 => Some(ClickHouseScalarType::Float32),
        ClickhouseDataType::Float64 => Some(ClickHouseScalarType::Float64),
        ClickhouseDataType::Decimal { .. } => Some(ClickHouseScalarType::Decimal),
        ClickhouseDataType::Decimal32 { .. } => Some(ClickHouseScalarType::Decimal32),
        ClickhouseDataType::Decimal64 { .. } => Some(ClickHouseScalarType::Decimal64),
        ClickhouseDataType::Decimal128 { .. } => Some(ClickHouseScalarType::Decimal128),
        ClickhouseDataType::Decimal256 { .. } => Some(ClickHouseScalarType::Decimal256),
        ClickhouseDataType::Date => Some(ClickHouseScalarType::Date),
        ClickhouseDataType::Date32 => Some(ClickHouseScalarType::Date32),
        ClickhouseDataType::DateTime { .. } => Some(ClickHouseScalarType::DateTime),
        ClickhouseDataType::DateTime64 { .. } => Some(ClickHouseScalarType::DateTime64),
        ClickhouseDataType::Json => Some(ClickHouseScalarType::Json),
        ClickhouseDataType::Uuid => Some(ClickHouseScalarType::Uuid),
        ClickhouseDataType::IPv4 => Some(ClickHouseScalarType::IPv4),
        ClickhouseDataType::IPv6 => Some(ClickHouseScalarType::IPv6),
        ClickhouseDataType::LowCardinality(data_type) => get_scalar_type(data_type),
        ClickhouseDataType::Nested(_)
        | ClickhouseDataType::Array(_)
        | ClickhouseDataType::Map { .. }
        | ClickhouseDataType::Tuple(_)
        | ClickhouseDataType::Enum(_) => None,
        ClickhouseDataType::SimpleAggregateFunction {
            function: _,
            arguments,
        }
        | ClickhouseDataType::AggregateFunction {
            function: _,
            arguments,
        } => arguments.first().and_then(get_scalar_type),
        ClickhouseDataType::Nothing => None,
    }
}

#[derive(Debug)]
pub enum TypeStringError {
    UnknownTable {
        table: String,
    },
    UnknownColumn {
        table: String,
        column: String,
    },
    CannotParseTypeString {
        table: String,
        column: String,
        data_type: String,
    },
    ColumnNotScalar {
        table: String,
        column: String,
        data_type: String,
    },
    UnknownAggregateFunction {
        table: String,
        column: String,
        data_type: String,
        scalar_type: String,
        function: String,
    },
    MissingRelationship(String),
}

impl Display for TypeStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeStringError::UnknownTable { table } => write!(f, "Unknown table: {table}"),
            TypeStringError::UnknownColumn { table, column } => {
                write!(f, "Unknown column: {column} in table: {table}")
            }
            TypeStringError::CannotParseTypeString {
                table,
                column,
                data_type,
            } => write!(
                f,
                "Unable to parse data type: {data_type} for column: {column} in table: {table}"
            ),
            TypeStringError::ColumnNotScalar {
                table,
                column,
                data_type,
            } => write!(
                f,
                "Unable to determine scalar type for column: {column} of type: {data_type} in table: {table}"
            ),
            TypeStringError::UnknownAggregateFunction {
                table,
                column,
                data_type,
                scalar_type,
                function,
            } => write!(f, "Unknown aggregate function: {function} for scalar type: {scalar_type} for column {column} of type: {data_type} in table {table}"),
            TypeStringError::MissingRelationship(rel) => write!(f, "Missing relationship: {rel}"),
        }
    }
}

impl std::error::Error for TypeStringError {}
