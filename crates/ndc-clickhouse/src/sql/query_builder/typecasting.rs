use std::{collections::BTreeMap, fmt::Display, str::FromStr};

use common::{clickhouse_parser::datatype::ClickHouseDataType, config::ServerConfig};
use indexmap::IndexMap;
use ndc_sdk::models;

use crate::schema::{ClickHouseSingleColumnAggregateFunction, ClickHouseTypeDefinition};

use super::QueryBuilderError;

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
                        let column_type = get_column(column_alias, table_alias, config)?;
                        let type_definition = ClickHouseTypeDefinition::from_table_column(
                            &column_type,
                            column_alias,
                            table_alias,
                        );

                        let aggregate_function =
                            ClickHouseSingleColumnAggregateFunction::from_str(&function).map_err(
                                |_err| TypeStringError::UnknownAggregateFunction {
                                    table: table_alias.to_owned(),
                                    column: column_alias.to_owned(),
                                    data_type: column_type.to_owned(),
                                    function: function.to_owned(),
                                },
                            )?;

                        let aggregate_functions = type_definition.aggregate_functions();

                        let result_type = aggregate_functions
                            .iter()
                            .find(|(function, _)| function == &aggregate_function)
                            .map(|(_, result_type)| result_type)
                            .ok_or_else(|| TypeStringError::UnknownAggregateFunction {
                                table: table_alias.to_owned(),
                                column: column_alias.to_owned(),
                                data_type: column_type.to_owned(),
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
                                fields,
                            } => {
                                if fields.is_some() {
                                    return Err(TypeStringError::NotSupported(
                                        "subfield selector".into(),
                                    ));
                                }
                                let column_type = get_column(column_alias, table_alias, config)?;
                                let type_definition = ClickHouseTypeDefinition::from_table_column(
                                    &column_type,
                                    column_alias,
                                    table_alias,
                                );
                                FieldTypeString::Column(type_definition.cast_type().to_string())
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
) -> Result<&'a ClickHouseDataType, TypeStringError> {
    let return_type = config
        .tables
        .get(table_alias)
        .map(|table| &table.return_type)
        .or_else(|| {
            config
                .queries
                .get(table_alias)
                .map(|query| &query.return_type)
        })
        .ok_or_else(|| TypeStringError::UnknownTable {
            table: table_alias.to_owned(),
        })?;

    let table_type =
        config
            .table_types
            .get(return_type)
            .ok_or_else(|| TypeStringError::UnknownTableType {
                table: return_type.to_owned(),
            })?;

    let column =
        table_type
            .columns
            .get(column_alias)
            .ok_or_else(|| TypeStringError::UnknownColumn {
                table: table_alias.to_owned(),
                column: column_alias.to_owned(),
            })?;

    Ok(column)
}

#[derive(Debug)]
pub enum TypeStringError {
    UnknownTable {
        table: String,
    },
    UnknownTableType {
        table: String,
    },
    UnknownColumn {
        table: String,
        column: String,
    },
    UnknownAggregateFunction {
        table: String,
        column: String,
        data_type: ClickHouseDataType,
        function: String,
    },
    MissingRelationship(String),
    NotSupported(String),
}

impl Display for TypeStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeStringError::UnknownTable { table } => write!(f, "Unknown table: {table}"),
            TypeStringError::UnknownTableType { table } => write!(f, "Unknown table type: {table}"),
            TypeStringError::UnknownColumn { table, column } => {
                write!(f, "Unknown column: {column} in table: {table}")
            }
            TypeStringError::UnknownAggregateFunction {
                table,
                column,
                data_type,
                function,
            } => write!(f, "Unknown aggregate function: {function} for column {column} of type: {data_type} in table {table}"),
            TypeStringError::MissingRelationship(rel) => write!(f, "Missing relationship: {rel}"),
            TypeStringError::NotSupported(feature) => write!(f, "Not supported: {feature}"),
        }
    }
}

impl std::error::Error for TypeStringError {}

impl From<TypeStringError> for QueryBuilderError {
    fn from(value: TypeStringError) -> Self {
        QueryBuilderError::Typecasting(value)
    }
}
