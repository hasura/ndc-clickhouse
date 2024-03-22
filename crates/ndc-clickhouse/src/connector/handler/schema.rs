use config::{PrimaryKey, ServerConfig};
use ndc_sdk::{connector::SchemaError, models};
use std::{collections::BTreeMap, str::FromStr};
use strum::IntoEnumIterator;

use crate::schema::{ClickHouseScalarType, ClickhouseDataType, Identifier};

pub async fn schema(configuration: &ServerConfig) -> Result<models::SchemaResponse, SchemaError> {
    let scalar_types = BTreeMap::from_iter(ClickHouseScalarType::iter().map(|scalar_type| {
        let aggregate_functions =
            BTreeMap::from_iter(scalar_type.aggregate_functions().into_iter().map(|(f, r)| {
                (
                    f.to_string(),
                    models::AggregateFunctionDefinition {
                        result_type: models::Type::Named {
                            name: r.to_string(),
                        },
                    },
                )
            }));
        (
            scalar_type.to_string(),
            models::ScalarType {
                aggregate_functions,
                comparison_operators: scalar_type.comparison_operators(),
            },
        )
    }));

    let mut object_types = vec![];

    for table in &configuration.tables {
        let mut fields = vec![];

        for column in &table.columns {
            let column_type = ClickhouseDataType::from_str(column.data_type.as_str())
                .map_err(|err| SchemaError::Other(Box::new(err)))?;

            let object_type_name = format!("{}_{}", table.alias, column.alias);

            let (column_type, additional_object_types) =
                get_field_type(&object_type_name, &column_type);

            for additional_object_type in additional_object_types {
                object_types.push(additional_object_type);
            }

            fields.push((
                column.alias.to_owned(),
                models::ObjectField {
                    description: None,
                    r#type: column_type,
                },
            ));
        }

        object_types.push((
            table.alias.to_owned(),
            models::ObjectType {
                description: None,
                fields: BTreeMap::from_iter(fields),
            },
        ));
    }

    let object_types = BTreeMap::from_iter(object_types);

    let collections = configuration
        .tables
        .iter()
        .map(|table| models::CollectionInfo {
            name: table.alias.to_owned(),
            description: None,
            arguments: BTreeMap::new(),
            collection_type: table.alias.to_owned(),
            uniqueness_constraints: table.primary_key.as_ref().map_or(
                BTreeMap::new(),
                |PrimaryKey { name, columns }| {
                    BTreeMap::from([(
                        name.to_owned(),
                        models::UniquenessConstraint {
                            unique_columns: columns.to_owned(),
                        },
                    )])
                },
            ),
            foreign_keys: BTreeMap::new(),
        })
        .collect();

    Ok(models::SchemaResponse {
        scalar_types,
        object_types,
        collections,
        functions: vec![],
        procedures: vec![],
    })
}

fn get_field_type(
    type_name: &str,
    data_type: &ClickhouseDataType,
) -> (models::Type, Vec<(String, models::ObjectType)>) {
    use ClickHouseScalarType as SC;
    use ClickhouseDataType as DT;
    let scalar = |t: ClickHouseScalarType| {
        (
            models::Type::Named {
                name: t.to_string(),
            },
            vec![],
        )
    };
    match data_type {
        DT::Nullable(inner) => {
            let (underlying_type, additional_types) = get_field_type(type_name, inner);
            (
                models::Type::Nullable {
                    underlying_type: Box::new(underlying_type),
                },
                additional_types,
            )
        }
        DT::Bool => scalar(SC::Bool),
        DT::String | DT::FixedString(_) => scalar(SC::String),
        DT::UInt8 => scalar(SC::UInt8),
        DT::UInt16 => scalar(SC::UInt16),
        DT::UInt32 => scalar(SC::UInt32),
        DT::UInt64 => scalar(SC::UInt64),
        DT::UInt128 => scalar(SC::UInt128),
        DT::UInt256 => scalar(SC::UInt256),
        DT::Int8 => scalar(SC::Int8),
        DT::Int16 => scalar(SC::Int16),
        DT::Int32 => scalar(SC::Int32),
        DT::Int64 => scalar(SC::Int64),
        DT::Int128 => scalar(SC::Int128),
        DT::Int256 => scalar(SC::Int256),
        DT::Float32 => scalar(SC::Float32),
        DT::Float64 => scalar(SC::Float64),
        DT::Decimal { .. } => scalar(SC::Decimal),
        DT::Decimal32 { .. } => scalar(SC::Decimal32),
        DT::Decimal64 { .. } => scalar(SC::Decimal64),
        DT::Decimal128 { .. } => scalar(SC::Decimal128),
        DT::Decimal256 { .. } => scalar(SC::Decimal256),
        DT::Date => scalar(SC::Date),
        DT::Date32 => scalar(SC::Date32),
        DT::DateTime { .. } => scalar(SC::DateTime),
        DT::DateTime64 { .. } => scalar(SC::DateTime64),
        DT::Json => scalar(SC::Json),
        DT::Uuid => scalar(SC::Uuid),
        DT::IPv4 => scalar(SC::IPv4),
        DT::IPv6 => scalar(SC::IPv6),
        DT::LowCardinality(inner) => get_field_type(type_name, inner),
        DT::Nested(entries) => {
            let object_type_name = type_name;

            let mut object_type_fields = vec![];
            let mut additional_object_types = vec![];

            for (name, data_type) in entries {
                let field_name = match name {
                    Identifier::DoubleQuoted(n) => n,
                    Identifier::BacktickQuoted(n) => n,
                    Identifier::Unquoted(n) => n,
                };

                let type_name = format!("{}_{}", object_type_name, field_name);

                let (field_type, mut additional_types) = get_field_type(&type_name, data_type);

                additional_object_types.append(&mut additional_types);

                object_type_fields.push((
                    field_name.to_owned(),
                    models::ObjectField {
                        description: None,
                        r#type: field_type,
                    },
                ));
            }

            additional_object_types.push((
                object_type_name.to_string(),
                models::ObjectType {
                    description: None,
                    fields: BTreeMap::from_iter(object_type_fields),
                },
            ));

            (
                models::Type::Named {
                    name: object_type_name.to_owned(),
                },
                additional_object_types,
            )
        }
        DT::Array(inner) => {
            let (inner, object_types) = get_field_type(type_name, inner);
            (
                models::Type::Array {
                    element_type: Box::new(inner),
                },
                object_types,
            )
        }
        DT::Map { key: _, value: _ } => scalar(SC::Unknown),
        DT::Tuple(entries) => {
            let object_type_name = type_name;

            let mut object_type_fields = vec![];
            let mut additional_object_types = vec![];

            for (name, data_type) in entries {
                let field_name = if let Some(name) = name {
                    match name {
                        Identifier::DoubleQuoted(n) => n,
                        Identifier::BacktickQuoted(n) => n,
                        Identifier::Unquoted(n) => n,
                    }
                } else {
                    return scalar(SC::Unknown);
                };

                let type_name = format!("{}_{}", object_type_name, field_name);

                let (field_type, mut additional_types) = get_field_type(&type_name, data_type);

                additional_object_types.append(&mut additional_types);

                object_type_fields.push((
                    field_name.to_owned(),
                    models::ObjectField {
                        description: None,
                        r#type: field_type,
                    },
                ));
            }

            additional_object_types.push((
                object_type_name.to_string(),
                models::ObjectType {
                    description: None,
                    fields: BTreeMap::from_iter(object_type_fields),
                },
            ));

            (
                models::Type::Named {
                    name: object_type_name.to_owned(),
                },
                additional_object_types,
            )
        }
        DT::Enum(_) => scalar(SC::String),
        DT::AggregateFunction {
            function,
            arguments,
        } => {
            let arg_len = arguments.len();
            let first = arguments.first();
            let agg_fn_name = match &function.name {
                Identifier::DoubleQuoted(n) => n,
                Identifier::BacktickQuoted(n) => n,
                Identifier::Unquoted(n) => n,
            };

            if let (Some(data_type), 1) = (first, arg_len) {
                get_field_type(type_name, data_type)
            } else if let (Some(data_type), 2, "anyIf") = (first, arg_len, agg_fn_name.as_str()) {
                get_field_type(type_name, data_type)
            } else {
                scalar(SC::Unknown)
            }
        }
        DT::SimpleAggregateFunction {
            function: _,
            arguments,
        } => {
            if let (Some(data_type), 1) = (arguments.first(), arguments.len()) {
                get_field_type(type_name, data_type)
            } else {
                scalar(SC::Unknown)
            }
        }
        DT::Nothing => scalar(SC::Unknown),
    }
}
