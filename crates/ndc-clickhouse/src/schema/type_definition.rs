use std::collections::BTreeMap;

use common::{
    clickhouse_parser::datatype::{ClickHouseDataType, Identifier, SingleQuotedString},
    config::ParameterizedQueryReturnType,
};
use ndc_sdk::models;

use super::{ClickHouseBinaryComparisonOperator, ClickHouseSingleColumnAggregateFunction};

#[derive(Debug, Clone, strum::Display)]
pub enum ClickHouseScalar {
    Bool,
    String,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    UInt256,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    Int256,
    Float32,
    Float64,
    Decimal,
    Decimal32,
    Decimal64,
    Decimal128,
    Decimal256,
    Date,
    Date32,
    DateTime,
    DateTime64,
    #[strum(to_string = "JSON")]
    Json,
    #[strum(to_string = "UUID")]
    Uuid,
    IPv4,
    IPv6,
    #[strum(to_string = "{name}")]
    Enum {
        name: String,
        variants: Vec<String>,
    },
}

impl ClickHouseScalar {
    fn type_name(&self) -> String {
        self.to_string()
    }
    fn type_definition(&self) -> models::ScalarType {
        models::ScalarType {
            representation: Some(self.json_representation()),
            aggregate_functions: self
                .aggregate_functions()
                .into_iter()
                .map(|(function, result_type)| {
                    (
                        function.to_string(),
                        models::AggregateFunctionDefinition {
                            result_type: models::Type::Named {
                                name: result_type.type_name(),
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
    fn json_representation(&self) -> models::TypeRepresentation {
        use models::TypeRepresentation as Rep;
        use ClickHouseScalar as ST;
        match self {
            ST::Bool => Rep::Boolean,
            ST::String => Rep::String,
            ST::UInt8 => Rep::Integer,
            ST::UInt16 => Rep::Integer,
            ST::UInt32 => Rep::Integer,
            ST::UInt64 => Rep::Integer,
            ST::UInt128 => Rep::Integer,
            ST::UInt256 => Rep::Integer,
            ST::Int8 => Rep::Integer,
            ST::Int16 => Rep::Integer,
            ST::Int32 => Rep::Integer,
            ST::Int64 => Rep::Integer,
            ST::Int128 => Rep::Integer,
            ST::Int256 => Rep::Integer,
            ST::Float32 => Rep::Number,
            ST::Float64 => Rep::Number,
            ST::Decimal => Rep::Number,
            ST::Decimal32 => Rep::String,
            ST::Decimal64 => Rep::String,
            ST::Decimal128 => Rep::String,
            ST::Decimal256 => Rep::String,
            ST::Date => Rep::String,
            ST::Date32 => Rep::String,
            ST::DateTime => Rep::String,
            ST::DateTime64 => Rep::String,
            ST::Json => Rep::String,
            ST::Uuid => Rep::String,
            ST::IPv4 => Rep::String,
            ST::IPv6 => Rep::String,
            ST::Enum { name: _, variants } => Rep::Enum {
                one_of: variants.to_owned(),
            },
        }
    }
    fn aggregate_functions(
        &self,
    ) -> Vec<(ClickHouseSingleColumnAggregateFunction, ClickHouseScalar)> {
        use ClickHouseScalar as ST;
        use ClickHouseSingleColumnAggregateFunction as AF;

        match self {
            ST::Bool => vec![],
            ST::String => vec![],
            ST::UInt8 => vec![
                (AF::Max, ST::UInt8),
                (AF::Min, ST::UInt8),
                (AF::Sum, ST::UInt64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::UInt16 => vec![
                (AF::Max, ST::UInt16),
                (AF::Min, ST::UInt16),
                (AF::Sum, ST::UInt64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::UInt32 => vec![
                (AF::Max, ST::UInt32),
                (AF::Min, ST::UInt32),
                (AF::Sum, ST::UInt64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::UInt64 => vec![
                (AF::Max, ST::UInt64),
                (AF::Min, ST::UInt64),
                (AF::Sum, ST::UInt64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::UInt128 => vec![
                (AF::Max, ST::UInt128),
                (AF::Min, ST::UInt128),
                (AF::Sum, ST::UInt128),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::UInt256 => vec![
                (AF::Max, ST::UInt256),
                (AF::Min, ST::UInt256),
                (AF::Sum, ST::UInt256),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Int8 => vec![
                (AF::Max, ST::Int8),
                (AF::Min, ST::Int8),
                (AF::Sum, ST::Int64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Int16 => vec![
                (AF::Max, ST::Int16),
                (AF::Min, ST::Int16),
                (AF::Sum, ST::Int64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Int32 => vec![
                (AF::Max, ST::Int32),
                (AF::Min, ST::Int32),
                (AF::Sum, ST::Int64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Int64 => vec![
                (AF::Max, ST::Int64),
                (AF::Min, ST::Int64),
                (AF::Sum, ST::Int64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Int128 => vec![
                (AF::Max, ST::Int128),
                (AF::Min, ST::Int128),
                (AF::Sum, ST::Int128),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Int256 => vec![
                (AF::Max, ST::Int256),
                (AF::Min, ST::Int256),
                (AF::Sum, ST::Int256),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Float32 => vec![
                (AF::Max, ST::Float64),
                (AF::Min, ST::Float32),
                (AF::Sum, ST::Float32),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float32),
                (AF::StddevSamp, ST::Float32),
                (AF::VarPop, ST::Float32),
                (AF::VarSamp, ST::Float32),
            ],
            ST::Float64 => vec![
                (AF::Max, ST::Float64),
                (AF::Min, ST::Float64),
                (AF::Sum, ST::Float64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Decimal => vec![
                (AF::Max, ST::Decimal),
                (AF::Min, ST::Decimal),
                (AF::Sum, ST::Decimal),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Decimal32 => vec![
                (AF::Max, ST::Decimal32),
                (AF::Min, ST::Decimal32),
                (AF::Sum, ST::Decimal32),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Decimal64 => vec![
                (AF::Max, ST::Decimal64),
                (AF::Min, ST::Decimal64),
                (AF::Sum, ST::Decimal64),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Decimal128 => vec![
                (AF::Max, ST::Decimal128),
                (AF::Min, ST::Decimal128),
                (AF::Sum, ST::Decimal128),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Decimal256 => vec![
                (AF::Max, ST::Decimal256),
                (AF::Min, ST::Decimal256),
                (AF::Sum, ST::Decimal256),
                (AF::Avg, ST::Float64),
                (AF::StddevPop, ST::Float64),
                (AF::StddevSamp, ST::Float64),
                (AF::VarPop, ST::Float64),
                (AF::VarSamp, ST::Float64),
            ],
            ST::Date => vec![(AF::Max, ST::Date), (AF::Min, ST::Date)],
            ST::Date32 => vec![(AF::Max, ST::Date32), (AF::Min, ST::Date32)],
            ST::DateTime => vec![(AF::Max, ST::DateTime), (AF::Min, ST::DateTime)],
            ST::DateTime64 => vec![(AF::Max, ST::DateTime64), (AF::Min, ST::DateTime64)],
            ST::Json => vec![],
            ST::Uuid => vec![],
            ST::IPv4 => vec![],
            ST::IPv6 => vec![],
            ST::Enum { .. } => vec![],
        }
    }
    fn comparison_operators(&self) -> Vec<ClickHouseBinaryComparisonOperator> {
        use ClickHouseBinaryComparisonOperator as BC;
        use ClickHouseScalar as ST;

        let equality_operators = vec![BC::Eq, BC::NotEq, BC::In, BC::NotIn];
        let ordering_operators = vec![BC::Gt, BC::Lt, BC::GtEq, BC::LtEq];
        let string_operators = vec![BC::Like, BC::NotLike, BC::ILike, BC::NotILike, BC::Match];

        match self {
            ST::Bool => equality_operators,
            ST::String => [equality_operators, ordering_operators, string_operators].concat(),
            ST::UInt8 | ST::UInt16 | ST::UInt32 | ST::UInt64 | ST::UInt128 | ST::UInt256 => {
                [equality_operators, ordering_operators].concat()
            }
            ST::Int8 | ST::Int16 | ST::Int32 | ST::Int64 | ST::Int128 | ST::Int256 => {
                [equality_operators, ordering_operators].concat()
            }
            ST::Float32 | ST::Float64 => [equality_operators, ordering_operators].concat(),
            ST::Decimal | ST::Decimal32 | ST::Decimal64 | ST::Decimal128 | ST::Decimal256 => {
                [equality_operators, ordering_operators].concat()
            }
            ST::Date | ST::Date32 => [equality_operators, ordering_operators].concat(),
            ST::DateTime | ST::DateTime64 => [equality_operators, ordering_operators].concat(),
            ST::Json => [equality_operators, ordering_operators].concat(),
            ST::Uuid => [equality_operators, ordering_operators].concat(),
            ST::IPv4 => [equality_operators, ordering_operators].concat(),
            ST::IPv6 => [equality_operators, ordering_operators].concat(),
            ST::Enum { .. } => equality_operators,
        }
    }
    /// returns the type we can cast this type to
    /// this may not be the same as the underlying real type
    /// for examples, enums are cast to strings, and fixed strings cast to strings
    fn cast_type(&self) -> ClickHouseDataType {
        match self {
            ClickHouseScalar::Bool => ClickHouseDataType::Bool,
            ClickHouseScalar::String => ClickHouseDataType::String,
            ClickHouseScalar::UInt8 => ClickHouseDataType::UInt8,
            ClickHouseScalar::UInt16 => ClickHouseDataType::UInt16,
            ClickHouseScalar::UInt32 => ClickHouseDataType::UInt32,
            ClickHouseScalar::UInt64 => ClickHouseDataType::UInt64,
            ClickHouseScalar::UInt128 => ClickHouseDataType::UInt128,
            ClickHouseScalar::UInt256 => ClickHouseDataType::UInt256,
            ClickHouseScalar::Int8 => ClickHouseDataType::Int8,
            ClickHouseScalar::Int16 => ClickHouseDataType::Int16,
            ClickHouseScalar::Int32 => ClickHouseDataType::Int32,
            ClickHouseScalar::Int64 => ClickHouseDataType::Int64,
            ClickHouseScalar::Int128 => ClickHouseDataType::Int128,
            ClickHouseScalar::Int256 => ClickHouseDataType::Int256,
            ClickHouseScalar::Float32 => ClickHouseDataType::Float32,
            ClickHouseScalar::Float64 => ClickHouseDataType::Float64,
            ClickHouseScalar::Decimal => ClickHouseDataType::String,
            ClickHouseScalar::Decimal32 => ClickHouseDataType::String,
            ClickHouseScalar::Decimal64 => ClickHouseDataType::String,
            ClickHouseScalar::Decimal128 => ClickHouseDataType::String,
            ClickHouseScalar::Decimal256 => ClickHouseDataType::String,
            ClickHouseScalar::Date => ClickHouseDataType::String,
            ClickHouseScalar::Date32 => ClickHouseDataType::String,
            ClickHouseScalar::DateTime => ClickHouseDataType::String,
            ClickHouseScalar::DateTime64 => ClickHouseDataType::String,
            ClickHouseScalar::Json => ClickHouseDataType::Json,
            ClickHouseScalar::Uuid => ClickHouseDataType::Uuid,
            ClickHouseScalar::IPv4 => ClickHouseDataType::IPv4,
            ClickHouseScalar::IPv6 => ClickHouseDataType::IPv6,
            ClickHouseScalar::Enum { .. } => ClickHouseDataType::String,
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
        fields: BTreeMap<String, ClickHouseTypeDefinition>,
    },
    /// Stand-in for data types that either cannot be represented in graphql,
    /// (such as maps or tuples with anonymous memebers)
    /// or cannot be known ahead of time (such as the return type of aggregate function columns)
    Unknown {
        name: String,
    },
}

impl ClickHouseTypeDefinition {
    /// Table alias is guaranteed unique across the database, and by default includes the schema name for non default schemas
    pub fn from_table_column(
        data_type: &ClickHouseDataType,
        column_alias: &str,
        table_alias: &str,
    ) -> Self {
        let namespace = format!("{table_alias}.{column_alias}");
        Self::new(data_type, &namespace)
    }
    pub fn from_query_return_type(
        data_type: &ClickHouseDataType,
        field_alias: &str,
        query_alias: &str,
    ) -> Self {
        let namespace = format!("{query_alias}.{field_alias}");
        Self::new(data_type, &namespace)
    }
    pub fn from_query_argument(
        data_type: &ClickHouseDataType,
        argument_alias: &str,
        query_alias: &str,
    ) -> Self {
        let namespace = format!("{query_alias}.arg.{argument_alias}");
        Self::new(data_type, &namespace)
    }
    fn new(data_type: &ClickHouseDataType, namespace: &str) -> Self {
        match data_type {
            ClickHouseDataType::Nullable(inner) => Self::Nullable {
                inner: Box::new(Self::new(inner, namespace)),
            },
            ClickHouseDataType::Bool => Self::Scalar(ClickHouseScalar::Bool),
            ClickHouseDataType::String | ClickHouseDataType::FixedString(_) => {
                Self::Scalar(ClickHouseScalar::String)
            }
            ClickHouseDataType::UInt8 => Self::Scalar(ClickHouseScalar::UInt8),
            ClickHouseDataType::UInt16 => Self::Scalar(ClickHouseScalar::UInt16),
            ClickHouseDataType::UInt32 => Self::Scalar(ClickHouseScalar::UInt32),
            ClickHouseDataType::UInt64 => Self::Scalar(ClickHouseScalar::UInt64),
            ClickHouseDataType::UInt128 => Self::Scalar(ClickHouseScalar::UInt128),
            ClickHouseDataType::UInt256 => Self::Scalar(ClickHouseScalar::UInt256),
            ClickHouseDataType::Int8 => Self::Scalar(ClickHouseScalar::Int8),
            ClickHouseDataType::Int16 => Self::Scalar(ClickHouseScalar::Int16),
            ClickHouseDataType::Int32 => Self::Scalar(ClickHouseScalar::Int32),
            ClickHouseDataType::Int64 => Self::Scalar(ClickHouseScalar::Int64),
            ClickHouseDataType::Int128 => Self::Scalar(ClickHouseScalar::Int128),
            ClickHouseDataType::Int256 => Self::Scalar(ClickHouseScalar::Int256),
            ClickHouseDataType::Float32 => Self::Scalar(ClickHouseScalar::Float32),
            ClickHouseDataType::Float64 => Self::Scalar(ClickHouseScalar::Float64),
            ClickHouseDataType::Decimal { .. } => Self::Scalar(ClickHouseScalar::Decimal),
            ClickHouseDataType::Decimal32 { .. } => Self::Scalar(ClickHouseScalar::Decimal32),
            ClickHouseDataType::Decimal64 { .. } => Self::Scalar(ClickHouseScalar::Decimal64),
            ClickHouseDataType::Decimal128 { .. } => Self::Scalar(ClickHouseScalar::Decimal128),
            ClickHouseDataType::Decimal256 { .. } => Self::Scalar(ClickHouseScalar::Decimal256),
            ClickHouseDataType::Date => Self::Scalar(ClickHouseScalar::Date),
            ClickHouseDataType::Date32 => Self::Scalar(ClickHouseScalar::Date32),
            ClickHouseDataType::DateTime { .. } => Self::Scalar(ClickHouseScalar::DateTime),
            ClickHouseDataType::DateTime64 { .. } => Self::Scalar(ClickHouseScalar::DateTime64),
            ClickHouseDataType::Json => Self::Scalar(ClickHouseScalar::Json),
            ClickHouseDataType::Uuid => Self::Scalar(ClickHouseScalar::Uuid),
            ClickHouseDataType::IPv4 => Self::Scalar(ClickHouseScalar::IPv4),
            ClickHouseDataType::IPv6 => Self::Scalar(ClickHouseScalar::IPv6),
            ClickHouseDataType::LowCardinality(inner) => Self::new(inner, namespace),
            ClickHouseDataType::Nested(entries) => {
                let mut fields = BTreeMap::new();

                for (name, field_data_type) in entries {
                    let field_name = match name {
                        Identifier::DoubleQuoted(n) => n,
                        Identifier::BacktickQuoted(n) => n,
                        Identifier::Unquoted(n) => n,
                    };

                    let field_namespace = format!("{namespace}_{field_name}");

                    let field_definition = Self::new(field_data_type, &field_namespace);

                    if fields
                        .insert(field_name.to_owned(), field_definition)
                        .is_some()
                    {
                        // on duplicate field names, fall back to unknown type
                        return Self::Unknown {
                            name: namespace.to_owned(),
                        };
                    }
                }

                Self::Object {
                    name: namespace.to_owned(),
                    fields,
                }
            }
            ClickHouseDataType::Array(element) => Self::Array {
                element_type: Box::new(Self::new(element, namespace)),
            },
            ClickHouseDataType::Map { .. } => Self::Unknown {
                name: namespace.to_owned(),
            },
            ClickHouseDataType::Tuple(entries) => {
                let mut fields = BTreeMap::new();

                for (name, field_data_type) in entries {
                    let field_name = if let Some(name) = name {
                        match name {
                            Identifier::DoubleQuoted(n) => n,
                            Identifier::BacktickQuoted(n) => n,
                            Identifier::Unquoted(n) => n,
                        }
                    } else {
                        return Self::Unknown {
                            name: namespace.to_owned(),
                        };
                    };

                    let field_namespace = format!("{namespace}.{field_name}");

                    let field_definition = Self::new(field_data_type, &field_namespace);

                    if fields
                        .insert(field_name.to_owned(), field_definition)
                        .is_some()
                    {
                        // on duplicate field names, fall back to unknown type
                        return Self::Unknown {
                            name: namespace.to_owned(),
                        };
                    }
                }

                Self::Object {
                    name: namespace.to_owned(),
                    fields,
                }
            }
            ClickHouseDataType::Enum(variants) => {
                let name = namespace.to_owned();
                let variants = variants
                    .iter()
                    .map(|(SingleQuotedString(variant), _)| variant.to_owned())
                    .collect();

                Self::Scalar(ClickHouseScalar::Enum { name, variants })
            }

            ClickHouseDataType::SimpleAggregateFunction {
                function: _,
                arguments,
            } => {
                if let (Some(data_type), 1) = (arguments.first(), arguments.len()) {
                    Self::new(data_type, namespace)
                } else {
                    Self::Unknown {
                        name: namespace.to_owned(),
                    }
                }
            }
            ClickHouseDataType::AggregateFunction {
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
                    Self::new(data_type, namespace)
                } else if let (Some(data_type), 2, "anyIf") = (first, arg_len, agg_fn_name.as_str())
                {
                    Self::new(data_type, namespace)
                } else {
                    Self::Unknown {
                        name: namespace.to_owned(),
                    }
                }
            }
            ClickHouseDataType::Nothing => Self::Unknown {
                name: namespace.to_owned(),
            },
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
            ClickHouseTypeDefinition::Unknown { name } => models::Type::Named {
                name: name.to_owned(),
            },
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
            ClickHouseTypeDefinition::Unknown { name } => {
                let definition = models::ScalarType {
                    representation: None,
                    aggregate_functions: BTreeMap::new(),
                    comparison_operators: BTreeMap::new(),
                };
                (vec![(name.to_owned(), definition)], vec![])
            }
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
                ClickHouseDataType::Nested(
                    fields
                        .iter()
                        .map(|(key, value)| {
                            // todo: prevent issues where the key contains unescaped double quotes
                            (Identifier::DoubleQuoted(key.to_owned()), value.cast_type())
                        })
                        .collect(),
                )
            }
            ClickHouseTypeDefinition::Unknown { .. } => ClickHouseDataType::Json,
        }
    }
    pub fn aggregate_functions(
        &self,
    ) -> Vec<(ClickHouseSingleColumnAggregateFunction, ClickHouseScalar)> {
        match self {
            ClickHouseTypeDefinition::Scalar(scalar) => scalar.aggregate_functions(),
            ClickHouseTypeDefinition::Nullable { inner } => inner.aggregate_functions(),
            ClickHouseTypeDefinition::Array { .. } => vec![],
            ClickHouseTypeDefinition::Object { .. } => vec![],
            ClickHouseTypeDefinition::Unknown { .. } => vec![],
        }
    }
}
