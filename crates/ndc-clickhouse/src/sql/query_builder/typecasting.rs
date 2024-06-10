use std::{collections::BTreeMap, fmt::Display, str::FromStr};

use common::{
    clickhouse_parser::datatype::{ClickHouseDataType, Identifier},
    config::ServerConfig,
};
use indexmap::IndexMap;
use ndc_sdk::models::{self, NestedField};

use crate::schema::{ClickHouseSingleColumnAggregateFunction, ClickHouseTypeDefinition};

use super::QueryBuilderError;

/// Tuple(rows <RowsCastString>, aggregates <RowsCastString>)
pub struct RowsetTypeString {
    rows: Option<RowTypeString>,
    aggregates: Option<AggregatesTypeString>,
}
/// Tuple("a1" T1, "a2" T2)
pub struct AggregatesTypeString {
    aggregates: Vec<(String, ClickHouseDataType)>,
}
/// Tuple("f1" T1, "f2" <RowSetTypeString>)
pub struct RowTypeString {
    fields: Vec<(String, FieldTypeString)>,
}
pub enum FieldTypeString {
    Relationship(RowsetTypeString),
    Array(Box<FieldTypeString>),
    Object(Vec<(String, FieldTypeString)>),
    Scalar(ClickHouseDataType),
}

impl RowsetTypeString {
    pub fn new(
        table_alias: &str,
        query: &models::Query,
        relationships: &BTreeMap<String, models::Relationship>,
        config: &ServerConfig,
    ) -> Result<Self, TypeStringError> {
        let rows = if let Some(fields) = &query.fields {
            Some(RowTypeString::new(
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
    pub fn into_cast_type(self) -> ClickHouseDataType {
        match (self.rows, self.aggregates) {
            (None, None) => ClickHouseDataType::Map {
                key: Box::new(ClickHouseDataType::Nothing),
                value: Box::new(ClickHouseDataType::Nothing),
            },
            (None, Some(aggregates)) => ClickHouseDataType::Tuple(vec![(
                Some(Identifier::Unquoted("aggregates".to_string())),
                aggregates.into_cast_type(),
            )]),
            (Some(rows), None) => ClickHouseDataType::Tuple(vec![(
                Some(Identifier::Unquoted("rows".to_string())),
                ClickHouseDataType::Array(Box::new(rows.into_cast_type()))
                )]),
                (Some(rows), Some(aggregates)) => ClickHouseDataType::Tuple(vec![
                    (
                        Some(Identifier::Unquoted("rows".to_string())),
                        ClickHouseDataType::Array(Box::new(rows.into_cast_type()))
                ),
                (
                    Some(Identifier::Unquoted("aggregates".to_string())),
                    aggregates.into_cast_type(),
                ),
            ]),
        }
    }
}

impl AggregatesTypeString {
    fn new(
        table_alias: &str,
        aggregates: &IndexMap<String, models::Aggregate>,
        config: &ServerConfig,
    ) -> Result<Self, TypeStringError> {
        Ok(Self {
            aggregates: aggregates
                .iter()
                .map(|(alias, aggregate)| match aggregate {
                    models::Aggregate::StarCount {} | models::Aggregate::ColumnCount { .. } => {
                        Ok((alias.to_string(), ClickHouseDataType::UInt32))
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
                            &config.namespace_separator,
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

                        Ok((alias.to_owned(), result_type.to_owned()))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
    fn into_cast_type(self) -> ClickHouseDataType {
        if self.aggregates.is_empty() {
            ClickHouseDataType::Map {
                key: Box::new(ClickHouseDataType::Nothing),
                value: Box::new(ClickHouseDataType::Nothing),
            }
        } else {
            ClickHouseDataType::Tuple(
                self.aggregates
                    .into_iter()
                    .map(|(alias, t)| (Some(Identifier::DoubleQuoted(alias)), t))
                    .collect(),
            )
        }
    }
}

impl RowTypeString {
    fn new(
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
                                let column_type = get_column(column_alias, table_alias, config)?;
                                let type_definition = ClickHouseTypeDefinition::from_table_column(
                                    &column_type,
                                    column_alias,
                                    table_alias,
                                    &config.namespace_separator,
                                );

                                FieldTypeString::new(
                                    &type_definition,
                                    fields.as_ref(),
                                    relationships,
                                    config,
                                )?
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
    fn into_cast_type(self) -> ClickHouseDataType {
        if self.fields.is_empty() {
            ClickHouseDataType::Map {
                key: Box::new(ClickHouseDataType::Nothing),
                value: Box::new(ClickHouseDataType::Nothing),
            }
        } else {
            ClickHouseDataType::Tuple(
                self.fields
                    .into_iter()
                    .map(|(alias, field)| {
                        (
                            Some(Identifier::DoubleQuoted(alias)),
                            field.into_cast_type(),
                        )
                    })
                    .collect(),
            )
        }
    }
}

impl FieldTypeString {
    fn new(
        type_definition: &ClickHouseTypeDefinition,
        fields: Option<&NestedField>,
        relationships: &BTreeMap<String, models::Relationship>,
        config: &ServerConfig,
    ) -> Result<Self, TypeStringError> {
        if let Some(fields) = fields {
            match (type_definition.non_nullable(), fields) {
                (
                    ClickHouseTypeDefinition::Array { element_type },
                    NestedField::Array(subfield_selector),
                ) => {
                    let type_definition = &**element_type;
                    let fields = Some(&*subfield_selector.fields);
                    let underlying_typestring =
                        FieldTypeString::new(type_definition, fields, relationships, config)?;
                    Ok(FieldTypeString::Array(Box::new(underlying_typestring)))
                }
                (
                    ClickHouseTypeDefinition::Object {
                        name: _,
                        fields,
                    },
                    NestedField::Object(subfield_selector),
                ) => {
                    let subfields = subfield_selector
                        .fields
                        .iter()
                        .map(|(alias, field)| {
                            match field {
                                models::Field::Column {
                                    column,
                                    fields: subfield_selector,
                                } => {
                                    let type_definition = fields.get(column).ok_or_else(|| {
                                        TypeStringError::MissingNestedField {
                                            field_name: column.to_owned(),
                                            object_type: type_definition.cast_type().to_string(),
                                        }
                                    })?;

                                    Ok((
                                        alias.to_owned(),
                                        FieldTypeString::new(
                                            &type_definition,
                                            subfield_selector.as_ref(),
                                            relationships,
                                            config,
                                        )?,
                                    ))
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

                                    Ok((
                                        alias.to_owned(),
                                        FieldTypeString::Relationship(RowsetTypeString::new(
                                            table_alias,
                                            query,
                                            relationships,
                                            config,
                                        )?),
                                    ))
                                }
                            }
                            // Ok((alias, FieldTypeString::new(type_definition, fields)))
                        })
                        .collect::<Result<_, _>>()?;
                    Ok(FieldTypeString::Object(subfields))
                }
                (ClickHouseTypeDefinition::Scalar(_), NestedField::Object(_)) => {
                    Err(TypeStringError::NestedFieldTypeMismatch {
                        expected: "Object".to_owned(),
                        got: type_definition.cast_type().to_string(),
                    })
                }
                (ClickHouseTypeDefinition::Scalar(_), NestedField::Array(_)) => {
                    Err(TypeStringError::NestedFieldTypeMismatch {
                        expected: "Array".to_owned(),
                        got: type_definition.cast_type().to_string(),
                    })
                }
                (ClickHouseTypeDefinition::Nullable { .. }, NestedField::Object(_)) => {
                    Err(TypeStringError::NestedFieldTypeMismatch {
                        expected: "Object".to_owned(),
                        got: type_definition.cast_type().to_string(),
                    })
                }
                (ClickHouseTypeDefinition::Nullable { .. }, NestedField::Array(_)) => {
                    Err(TypeStringError::NestedFieldTypeMismatch {
                        expected: "Array".to_owned(),
                        got: type_definition.cast_type().to_string(),
                    })
                }
                (ClickHouseTypeDefinition::Array { .. }, NestedField::Object(_)) => {
                    Err(TypeStringError::NestedFieldTypeMismatch {
                        expected: "Object".to_owned(),
                        got: type_definition.cast_type().to_string(),
                    })
                }
                (ClickHouseTypeDefinition::Object { .. }, NestedField::Array(_)) => {
                    Err(TypeStringError::NestedFieldTypeMismatch {
                        expected: "Array".to_owned(),
                        got: type_definition.cast_type().to_string(),
                    })
                }
            }
        } else {
            Ok(FieldTypeString::Scalar(type_definition.cast_type()))
        }
    }
    fn into_cast_type(self) -> ClickHouseDataType {
        match self {
            FieldTypeString::Relationship(rel) => rel.into_cast_type(),
            FieldTypeString::Array(inner) => {
                ClickHouseDataType::Array(Box::new(inner.into_cast_type()))
            }
            FieldTypeString::Object(fields) => ClickHouseDataType::Tuple(
                fields
                    .into_iter()
                    .map(|(alias, field)| {
                        (
                            Some(Identifier::DoubleQuoted(alias)),
                            field.into_cast_type(),
                        )
                    })
                    .collect(),
            ),
            FieldTypeString::Scalar(inner) => inner,
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

#[derive(Debug, PartialEq)]
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
    NestedFieldTypeMismatch {
        expected: String,
        got: String,
    },
    MissingNestedField {
        field_name: String,
        object_type: String,
    },
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
            TypeStringError::NestedFieldTypeMismatch { expected, got } => write!(f, "Nested field selector type mismatch, expected: {expected}, got {got}"),
            TypeStringError::MissingNestedField { field_name, object_type } => write!(f, "Missing field {field_name} in object type {object_type}"),
            
        }
    }
}

impl std::error::Error for TypeStringError {}

impl From<TypeStringError> for QueryBuilderError {
    fn from(value: TypeStringError) -> Self {
        QueryBuilderError::Typecasting(value)
    }
}
