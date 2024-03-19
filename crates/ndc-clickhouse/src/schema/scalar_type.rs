use std::collections::BTreeMap;

use crate::schema::{
    binary_comparison_operator::ClickHouseBinaryComparisonOperator,
    single_column_aggregate_function::ClickHouseSingleColumnAggregateFunction,
};
use ndc_sdk::models;
use strum::{Display, EnumIter, EnumString};

#[derive(Debug, Clone, EnumString, Display, EnumIter)]
pub enum ClickHouseScalarType {
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
    Json,
    Uuid,
    IPv4,
    IPv6,
    /// Stand-in for data types that either cannot be represented in graphql,
    /// (such as maps or tuples with anonymous memebers)
    /// or cannot be known ahead of time (such as the return type of aggregate function columns)
    Unknown,
}

impl ClickHouseScalarType {
    pub fn aggregate_functions(
        &self,
    ) -> Vec<(
        ClickHouseSingleColumnAggregateFunction,
        ClickHouseScalarType,
    )> {
        use ClickHouseScalarType as ST;
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
            ST::Unknown => vec![],
        }
    }
    pub fn comparison_operators(&self) -> BTreeMap<String, models::ComparisonOperatorDefinition> {
        use ClickHouseBinaryComparisonOperator as BC;
        use ClickHouseScalarType as ST;
        let base_operators = vec![
            (BC::Eq, self.to_owned()),
            (BC::Gt, self.to_owned()),
            (BC::Lt, self.to_owned()),
            (BC::GtEq, self.to_owned()),
            (BC::LtEq, self.to_owned()),
            (BC::NotEq, self.to_owned()),
            (BC::In, self.to_owned()),
            (BC::NotIn, self.to_owned()),
        ];

        BTreeMap::from_iter(
            match self {
                ST::String => vec![(BC::Like, ST::String)],
                _ => vec![],
            }
            .into_iter()
            .chain(base_operators)
            .map(|(name, argument_type)| {
                (
                    name.to_string(),
                    match name {
                        BC::Eq => models::ComparisonOperatorDefinition::Equal,
                        BC::In => models::ComparisonOperatorDefinition::In,
                        BC::NotIn => models::ComparisonOperatorDefinition::Custom {
                            argument_type: models::Type::Array {
                                element_type: Box::new(models::Type::Named {
                                    name: argument_type.to_string(),
                                }),
                            },
                        },
                        _ => models::ComparisonOperatorDefinition::Custom {
                            argument_type: models::Type::Named {
                                name: argument_type.to_string(),
                            },
                        },
                    },
                )
            }),
        )
    }
}
