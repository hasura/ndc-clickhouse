use common::clickhouse_parser::datatype::{ClickHouseDataType, Identifier, SingleQuotedString};
use indexmap::IndexMap;
use ndc_sdk::models;
use std::iter;

use super::{ClickHouseBinaryComparisonOperator, ClickHouseSingleColumnAggregateFunction};

#[derive(Debug, Clone)]
struct NameSpace<'a> {
    separator: &'a str,
    path: Vec<&'a str>,
}

impl<'a> NameSpace<'a> {
    pub fn new(path: Vec<&'a str>, separator: &'a str) -> Self {
        Self { separator, path }
    }
    pub fn value(&self) -> String {
        self.path.join(&self.separator)
    }
    pub fn child(&self, path_element: &'a str) -> Self {
        Self {
            separator: self.separator,
            path: self
                .path
                .clone()
                .into_iter()
                .chain(iter::once(path_element))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClickHouseScalar(ClickHouseDataType);

impl ClickHouseScalar {
    fn type_name(&self) -> String {
        self.0.to_string()
    }
    fn cast_type(&self) -> ClickHouseDataType {
        // todo: recusively map large number types to string here
        self.0.clone()
    }
    fn type_definition(&self) -> models::ScalarType {
        models::ScalarType {
            representation: self.json_representation(),
            aggregate_functions: self
                .aggregate_functions()
                .into_iter()
                .map(|(function, result_type)| {
                    (
                        function.to_string(),
                        models::AggregateFunctionDefinition {
                            result_type: models::Type::Named {
                                name: result_type.to_string(),
                            },
                        },
                    )
                })
                .collect(),
            comparison_operators: self
                .comparison_operators()
                .into_iter()
                .map(|operator| {
                    let definition = match operator {
                        ClickHouseBinaryComparisonOperator::Eq => {
                            models::ComparisonOperatorDefinition::Equal
                        }
                        ClickHouseBinaryComparisonOperator::In => {
                            models::ComparisonOperatorDefinition::In
                        }
                        ClickHouseBinaryComparisonOperator::NotIn => {
                            models::ComparisonOperatorDefinition::Custom {
                                argument_type: models::Type::Array {
                                    element_type: Box::new(models::Type::Named {
                                        name: self.type_name(),
                                    }),
                                },
                            }
                        }
                        ClickHouseBinaryComparisonOperator::Gt
                        | ClickHouseBinaryComparisonOperator::Lt
                        | ClickHouseBinaryComparisonOperator::GtEq
                        | ClickHouseBinaryComparisonOperator::LtEq
                        | ClickHouseBinaryComparisonOperator::NotEq
                        | ClickHouseBinaryComparisonOperator::Like
                        | ClickHouseBinaryComparisonOperator::NotLike
                        | ClickHouseBinaryComparisonOperator::ILike
                        | ClickHouseBinaryComparisonOperator::NotILike
                        | ClickHouseBinaryComparisonOperator::Match => {
                            models::ComparisonOperatorDefinition::Custom {
                                argument_type: models::Type::Named {
                                    name: self.type_name(),
                                },
                            }
                        }
                    };
                    (operator.to_string(), definition)
                })
                .collect(),
        }
    }
    fn json_representation(&self) -> Option<models::TypeRepresentation> {
        use models::TypeRepresentation as Rep;
        match &self.0 {
            ClickHouseDataType::Bool => Some(Rep::Boolean),
            ClickHouseDataType::String => Some(Rep::String),
            ClickHouseDataType::UInt8 => Some(Rep::Integer),
            ClickHouseDataType::UInt16 => Some(Rep::Integer),
            ClickHouseDataType::UInt32 => Some(Rep::Integer),
            ClickHouseDataType::UInt64 => Some(Rep::Integer),
            ClickHouseDataType::UInt128 => Some(Rep::Integer),
            ClickHouseDataType::UInt256 => Some(Rep::Integer),
            ClickHouseDataType::Int8 => Some(Rep::Integer),
            ClickHouseDataType::Int16 => Some(Rep::Integer),
            ClickHouseDataType::Int32 => Some(Rep::Integer),
            ClickHouseDataType::Int64 => Some(Rep::Integer),
            ClickHouseDataType::Int128 => Some(Rep::Integer),
            ClickHouseDataType::Int256 => Some(Rep::Integer),
            ClickHouseDataType::Float32 => Some(Rep::Number),
            ClickHouseDataType::Float64 => Some(Rep::Number),
            ClickHouseDataType::Decimal { .. } => Some(Rep::Number),
            ClickHouseDataType::Decimal32 { .. } => Some(Rep::String),
            ClickHouseDataType::Decimal64 { .. } => Some(Rep::String),
            ClickHouseDataType::Decimal128 { .. } => Some(Rep::String),
            ClickHouseDataType::Decimal256 { .. } => Some(Rep::String),
            ClickHouseDataType::Date => Some(Rep::String),
            ClickHouseDataType::Date32 => Some(Rep::String),
            ClickHouseDataType::DateTime { .. } => Some(Rep::String),
            ClickHouseDataType::DateTime64 { .. } => Some(Rep::String),
            ClickHouseDataType::Json => Some(Rep::String),
            ClickHouseDataType::Uuid => Some(Rep::String),
            ClickHouseDataType::IPv4 => Some(Rep::String),
            ClickHouseDataType::IPv6 => Some(Rep::String),
            ClickHouseDataType::Enum(variants) => {
                let variants = variants
                    .iter()
                    .map(|(SingleQuotedString(variant), _)| variant.to_owned())
                    .collect();

                Some(Rep::Enum { one_of: variants })
            }
            _ => None,
        }
    }
    fn aggregate_functions(
        &self,
    ) -> Vec<(ClickHouseSingleColumnAggregateFunction, ClickHouseDataType)> {
        use ClickHouseSingleColumnAggregateFunction as AF;

        match self.0 {
            ClickHouseDataType::Bool => vec![],
            ClickHouseDataType::String => vec![],
            ClickHouseDataType::UInt8 => vec![
                (AF::Max, ClickHouseDataType::UInt8),
                (AF::Min, ClickHouseDataType::UInt8),
                (AF::Sum, ClickHouseDataType::UInt64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::UInt16 => vec![
                (AF::Max, ClickHouseDataType::UInt16),
                (AF::Min, ClickHouseDataType::UInt16),
                (AF::Sum, ClickHouseDataType::UInt64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::UInt32 => vec![
                (AF::Max, ClickHouseDataType::UInt32),
                (AF::Min, ClickHouseDataType::UInt32),
                (AF::Sum, ClickHouseDataType::UInt64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::UInt64 => vec![
                (AF::Max, ClickHouseDataType::UInt64),
                (AF::Min, ClickHouseDataType::UInt64),
                (AF::Sum, ClickHouseDataType::UInt64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::UInt128 => vec![
                (AF::Max, ClickHouseDataType::UInt128),
                (AF::Min, ClickHouseDataType::UInt128),
                (AF::Sum, ClickHouseDataType::UInt128),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::UInt256 => vec![
                (AF::Max, ClickHouseDataType::UInt256),
                (AF::Min, ClickHouseDataType::UInt256),
                (AF::Sum, ClickHouseDataType::UInt256),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Int8 => vec![
                (AF::Max, ClickHouseDataType::Int8),
                (AF::Min, ClickHouseDataType::Int8),
                (AF::Sum, ClickHouseDataType::Int64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Int16 => vec![
                (AF::Max, ClickHouseDataType::Int16),
                (AF::Min, ClickHouseDataType::Int16),
                (AF::Sum, ClickHouseDataType::Int64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Int32 => vec![
                (AF::Max, ClickHouseDataType::Int32),
                (AF::Min, ClickHouseDataType::Int32),
                (AF::Sum, ClickHouseDataType::Int64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Int64 => vec![
                (AF::Max, ClickHouseDataType::Int64),
                (AF::Min, ClickHouseDataType::Int64),
                (AF::Sum, ClickHouseDataType::Int64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Int128 => vec![
                (AF::Max, ClickHouseDataType::Int128),
                (AF::Min, ClickHouseDataType::Int128),
                (AF::Sum, ClickHouseDataType::Int128),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Int256 => vec![
                (AF::Max, ClickHouseDataType::Int256),
                (AF::Min, ClickHouseDataType::Int256),
                (AF::Sum, ClickHouseDataType::Int256),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Float32 => vec![
                (AF::Max, ClickHouseDataType::Float64),
                (AF::Min, ClickHouseDataType::Float32),
                (AF::Sum, ClickHouseDataType::Float32),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float32),
                (AF::StddevSamp, ClickHouseDataType::Float32),
                (AF::VarPop, ClickHouseDataType::Float32),
                (AF::VarSamp, ClickHouseDataType::Float32),
            ],
            ClickHouseDataType::Float64 => vec![
                (AF::Max, ClickHouseDataType::Float64),
                (AF::Min, ClickHouseDataType::Float64),
                (AF::Sum, ClickHouseDataType::Float64),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Decimal { .. } => vec![
                (AF::Max, self.0.to_owned()),
                (AF::Min, self.0.to_owned()),
                (AF::Sum, self.0.to_owned()),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Decimal32 { .. } => vec![
                (AF::Max, self.0.to_owned()),
                (AF::Min, self.0.to_owned()),
                (AF::Sum, self.0.to_owned()),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Decimal64 { .. } => vec![
                (AF::Max, self.0.to_owned()),
                (AF::Min, self.0.to_owned()),
                (AF::Sum, self.0.to_owned()),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Decimal128 { .. } => vec![
                (AF::Max, self.0.to_owned()),
                (AF::Min, self.0.to_owned()),
                (AF::Sum, self.0.to_owned()),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Decimal256 { .. } => vec![
                (AF::Max, self.0.to_owned()),
                (AF::Min, self.0.to_owned()),
                (AF::Sum, self.0.to_owned()),
                (AF::Avg, ClickHouseDataType::Float64),
                (AF::StddevPop, ClickHouseDataType::Float64),
                (AF::StddevSamp, ClickHouseDataType::Float64),
                (AF::VarPop, ClickHouseDataType::Float64),
                (AF::VarSamp, ClickHouseDataType::Float64),
            ],
            ClickHouseDataType::Date => vec![
                (AF::Max, ClickHouseDataType::Date),
                (AF::Min, ClickHouseDataType::Date),
            ],
            ClickHouseDataType::Date32 => vec![
                (AF::Max, ClickHouseDataType::Date32),
                (AF::Min, ClickHouseDataType::Date32),
            ],
            ClickHouseDataType::DateTime { .. } => {
                vec![(AF::Max, self.0.to_owned()), (AF::Min, self.0.to_owned())]
            }
            ClickHouseDataType::DateTime64 { .. } => {
                vec![(AF::Max, self.0.to_owned()), (AF::Min, self.0.to_owned())]
            }
            _ => vec![],
        }
    }
    fn comparison_operators(&self) -> Vec<ClickHouseBinaryComparisonOperator> {
        use ClickHouseBinaryComparisonOperator as BC;

        let equality_operators = vec![BC::Eq, BC::NotEq, BC::In, BC::NotIn];
        let ordering_operators = vec![BC::Gt, BC::Lt, BC::GtEq, BC::LtEq];
        let string_operators = vec![BC::Like, BC::NotLike, BC::ILike, BC::NotILike, BC::Match];

        match self.0 {
            ClickHouseDataType::Bool => equality_operators,
            ClickHouseDataType::String => {
                [equality_operators, ordering_operators, string_operators].concat()
            }
            ClickHouseDataType::UInt8
            | ClickHouseDataType::UInt16
            | ClickHouseDataType::UInt32
            | ClickHouseDataType::UInt64
            | ClickHouseDataType::UInt128
            | ClickHouseDataType::UInt256 => [equality_operators, ordering_operators].concat(),
            ClickHouseDataType::Int8
            | ClickHouseDataType::Int16
            | ClickHouseDataType::Int32
            | ClickHouseDataType::Int64
            | ClickHouseDataType::Int128
            | ClickHouseDataType::Int256 => [equality_operators, ordering_operators].concat(),
            ClickHouseDataType::Float32 | ClickHouseDataType::Float64 => {
                [equality_operators, ordering_operators].concat()
            }
            ClickHouseDataType::Decimal { .. }
            | ClickHouseDataType::Decimal32 { .. }
            | ClickHouseDataType::Decimal64 { .. }
            | ClickHouseDataType::Decimal128 { .. }
            | ClickHouseDataType::Decimal256 { .. } => {
                [equality_operators, ordering_operators].concat()
            }
            ClickHouseDataType::Date | ClickHouseDataType::Date32 => {
                [equality_operators, ordering_operators].concat()
            }
            ClickHouseDataType::DateTime { .. } | ClickHouseDataType::DateTime64 { .. } => {
                [equality_operators, ordering_operators].concat()
            }
            ClickHouseDataType::Json => [equality_operators, ordering_operators].concat(),
            ClickHouseDataType::Uuid => [equality_operators, ordering_operators].concat(),
            ClickHouseDataType::IPv4 => [equality_operators, ordering_operators].concat(),
            ClickHouseDataType::IPv6 => [equality_operators, ordering_operators].concat(),
            ClickHouseDataType::Enum { .. } => equality_operators,
            _ => vec![],
        }
    }
}

pub enum ClickHouseTypeDefinition {
    Scalar(ClickHouseScalar),
    Nullable {
        inner: Box<ClickHouseTypeDefinition>,
    },
    Array {
        element_type: Box<ClickHouseTypeDefinition>,
    },
    Object {
        name: String,
        fields: IndexMap<String, ClickHouseTypeDefinition>,
    },
}

impl ClickHouseTypeDefinition {
    /// Table alias is guaranteed unique across the database, and by default includes the schema name for non default schemas
    pub fn from_table_column(
        data_type: &ClickHouseDataType,
        column_alias: &str,
        table_alias: &str,
        separator: &str,
    ) -> Self {
        let namespace = NameSpace::new(vec![table_alias, column_alias], separator);
        Self::new(data_type, &namespace)
    }
    pub fn from_query_return_type(
        data_type: &ClickHouseDataType,
        field_alias: &str,
        query_alias: &str,
        separator: &str,
    ) -> Self {
        let namespace = NameSpace::new(vec![query_alias, field_alias], separator);
        Self::new(data_type, &namespace)
    }
    pub fn from_query_argument(
        data_type: &ClickHouseDataType,
        argument_alias: &str,
        query_alias: &str,
        separator: &str,
    ) -> Self {
        let namespace = NameSpace::new(vec![query_alias, "_arg", argument_alias], separator);
        Self::new(data_type, &namespace)
    }
    fn new(data_type: &ClickHouseDataType, namespace: &NameSpace) -> Self {
        match data_type {
            ClickHouseDataType::Nullable(inner) => Self::Nullable {
                inner: Box::new(Self::new(inner, namespace)),
            },
            ClickHouseDataType::String | ClickHouseDataType::FixedString(_) => {
                Self::Scalar(ClickHouseScalar(ClickHouseDataType::String))
            }
            ClickHouseDataType::LowCardinality(inner) => Self::new(inner, namespace),
            ClickHouseDataType::Nested(entries) => {
                let mut fields = IndexMap::new();

                for (name, field_data_type) in entries {
                    let field_namespace = namespace.child(name.value());

                    let field_definition = Self::new(field_data_type, &field_namespace);

                    if fields
                        .insert(name.value().to_owned(), field_definition)
                        .is_some()
                    {
                        // on duplicate field names, fall back to unknown type
                        return Self::Scalar(ClickHouseScalar(data_type.to_owned()));
                    }
                }

                Self::Array {
                    element_type: Box::new(Self::Object {
                        name: namespace.value(),
                        fields,
                    }),
                }
            }
            ClickHouseDataType::Array(element) => Self::Array {
                element_type: Box::new(Self::new(element, namespace)),
            },
            ClickHouseDataType::Tuple(entries) => {
                let mut fields = IndexMap::new();

                for (name, field_data_type) in entries {
                    let field_name = if let Some(name) = name {
                        name.value()
                    } else {
                        // anonymous tuples treated as scalar types
                        return Self::Scalar(ClickHouseScalar(data_type.to_owned()));
                    };

                    let field_namespace = namespace.child(&field_name);

                    let field_definition = Self::new(field_data_type, &field_namespace);

                    if fields
                        .insert(field_name.to_owned(), field_definition)
                        .is_some()
                    {
                        // on duplicate field names, fall back to unknown type
                        return Self::Scalar(ClickHouseScalar(data_type.to_owned()));
                    }
                }

                Self::Object {
                    name: namespace.value(),
                    fields,
                }
            }
            ClickHouseDataType::SimpleAggregateFunction {
                function: _,
                arguments,
            } => {
                if let (Some(data_type), 1) = (arguments.first(), arguments.len()) {
                    Self::new(data_type, namespace)
                } else {
                    Self::Scalar(ClickHouseScalar(data_type.to_owned()))
                }
            }
            ClickHouseDataType::AggregateFunction {
                function,
                arguments,
            } => {
                let arg_len = arguments.len();
                let first = arguments.first();

                if let (Some(data_type), 1) = (first, arg_len) {
                    Self::new(data_type, namespace)
                } else if let (Some(data_type), 2, "anyIf") =
                    (first, arg_len, function.name.value())
                {
                    Self::new(data_type, namespace)
                } else {
                    Self::Scalar(ClickHouseScalar(data_type.to_owned()))
                }
            }
            _ => Self::Scalar(ClickHouseScalar(data_type.to_owned())),
        }
    }
    pub fn type_identifier(&self) -> models::Type {
        match self {
            ClickHouseTypeDefinition::Scalar(scalar) => models::Type::Named {
                name: scalar.type_name(),
            },
            ClickHouseTypeDefinition::Nullable { inner } => models::Type::Nullable {
                underlying_type: Box::new(inner.type_identifier()),
            },
            ClickHouseTypeDefinition::Array { element_type } => models::Type::Array {
                element_type: Box::new(element_type.type_identifier()),
            },
            ClickHouseTypeDefinition::Object { name, fields: _ } => models::Type::Named {
                name: name.to_owned(),
            },
        }
    }
    pub fn cast_type(&self) -> ClickHouseDataType {
        match self {
            ClickHouseTypeDefinition::Scalar(scalar) => scalar.cast_type(),
            ClickHouseTypeDefinition::Nullable { inner } => {
                ClickHouseDataType::Nullable(Box::new(inner.cast_type()))
            }
            ClickHouseTypeDefinition::Array { element_type } => {
                ClickHouseDataType::Array(Box::new(element_type.cast_type()))
            }
            ClickHouseTypeDefinition::Object { name: _, fields } => {
                ClickHouseDataType::Tuple(
                    fields
                        .iter()
                        .map(|(key, value)| {
                            // todo: prevent issues where the key contains unescaped double quotes
                            (
                                Some(Identifier::DoubleQuoted(key.to_owned())),
                                value.cast_type(),
                            )
                        })
                        .collect(),
                )
            }
        }
    }
    /// returns the schema type definitions for this type
    /// note that ScalarType definitions may be duplicated
    pub fn type_definitions(
        &self,
    ) -> (
        Vec<(String, models::ScalarType)>,
        Vec<(String, models::ObjectType)>,
    ) {
        match self {
            ClickHouseTypeDefinition::Scalar(scalar) => {
                (vec![(scalar.type_name(), scalar.type_definition())], vec![])
            }
            ClickHouseTypeDefinition::Nullable { inner } => inner.type_definitions(),
            ClickHouseTypeDefinition::Array { element_type } => element_type.type_definitions(),
            ClickHouseTypeDefinition::Object {
                name: namespace,
                fields,
            } => {
                let mut object_type_fields = vec![];
                let mut object_type_definitions = vec![];
                let mut scalar_type_definitions = vec![];

                for (field_name, field) in fields {
                    let (mut scalars, mut objects) = field.type_definitions();

                    scalar_type_definitions.append(&mut scalars);
                    object_type_definitions.append(&mut objects);

                    object_type_fields.push((
                        field_name.to_owned(),
                        models::ObjectField {
                            description: None,
                            r#type: field.type_identifier(),
                        },
                    ));
                }

                object_type_definitions.push((
                    namespace.to_string(),
                    models::ObjectType {
                        description: None,
                        fields: object_type_fields.into_iter().collect(),
                    },
                ));

                (scalar_type_definitions, object_type_definitions)
            }
        }
    }
    pub fn aggregate_functions(
        &self,
    ) -> Vec<(ClickHouseSingleColumnAggregateFunction, ClickHouseDataType)> {
        match self {
            ClickHouseTypeDefinition::Scalar(scalar) => scalar.aggregate_functions(),
            ClickHouseTypeDefinition::Nullable { inner } => inner.aggregate_functions(),
            ClickHouseTypeDefinition::Array { .. } => vec![],
            ClickHouseTypeDefinition::Object { .. } => vec![],
        }
    }
    /// the underlying non-nullable type, with any wrapping nullable variants removed
    pub fn non_nullable(&self) -> &Self {
        match self {
            ClickHouseTypeDefinition::Nullable { inner } => inner.non_nullable(),
            ClickHouseTypeDefinition::Scalar(_) => self,
            ClickHouseTypeDefinition::Array { .. } => self,
            ClickHouseTypeDefinition::Object { .. } => self,
        }
    }
}
