use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use super::clickhouse_parser;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingleQuotedString(pub String);

impl Display for SingleQuotedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Identifier {
    DoubleQuoted(String),
    BacktickQuoted(String),
    Unquoted(String),
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Identifier::DoubleQuoted(s) => write!(f, "\"{s}\""),
            Identifier::BacktickQuoted(s) => write!(f, "`{s}`"),
            Identifier::Unquoted(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateFunctionDefinition {
    pub name: Identifier,
    pub parameters: Option<Vec<AggregateFunctionParameter>>,
}

impl Display for AggregateFunctionDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;

        if let Some(parameters) = &self.parameters {
            write!(f, "(")?;
            let mut first = true;
            for parameter in parameters {
                if first {
                    first = false;
                } else {
                    write!(f, ", ")?;
                }
                write!(f, "{parameter}")?;
            }
            write!(f, ")")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateFunctionParameter {
    SingleQuotedString(SingleQuotedString),
    FloatingPoint(f64),
    Integer(u32),
}

impl Display for AggregateFunctionParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregateFunctionParameter::SingleQuotedString(s) => write!(f, "{s}"),
            AggregateFunctionParameter::FloatingPoint(n) => write!(f, "{n}"),
            AggregateFunctionParameter::Integer(n) => write!(f, "{n}"),
        }
    }
}

/// A parsed representation of a clickhouse datatype string
/// This should support the full scope of clickhouse types
/// To create one from a string slice, use try_from()
#[derive(Debug, Clone, PartialEq)]
pub enum ClickHouseDataType {
    Nullable(Box<ClickHouseDataType>),
    Bool,
    String,
    FixedString(u32),
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
    Decimal {
        precision: u32,
        scale: u32,
    },
    Decimal32 {
        scale: u32,
    },
    Decimal64 {
        scale: u32,
    },
    Decimal128 {
        scale: u32,
    },
    Decimal256 {
        scale: u32,
    },
    Date,
    Date32,
    DateTime {
        timezone: Option<SingleQuotedString>,
    },
    DateTime64 {
        precision: u32,
        timezone: Option<SingleQuotedString>,
    },
    Json,
    Uuid,
    IPv4,
    IPv6,
    LowCardinality(Box<ClickHouseDataType>),
    Nested(Vec<(Identifier, ClickHouseDataType)>),
    Array(Box<ClickHouseDataType>),
    Map {
        key: Box<ClickHouseDataType>,
        value: Box<ClickHouseDataType>,
    },
    Tuple(Vec<(Option<Identifier>, ClickHouseDataType)>),
    Enum(Vec<(SingleQuotedString, Option<u32>)>),
    SimpleAggregateFunction {
        function: AggregateFunctionDefinition,
        arguments: Vec<ClickHouseDataType>,
    },
    AggregateFunction {
        function: AggregateFunctionDefinition,
        arguments: Vec<ClickHouseDataType>,
    },
    Nothing,
    CompoundIdentifier(Vec<Identifier>),
    SingleIdentifier(Identifier),
}

impl Display for ClickHouseDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ClickHouseDataType as DT;
        match self {
            DT::Nullable(inner) => write!(f, "Nullable({inner})"),
            DT::Bool => write!(f, "Bool"),
            DT::String => write!(f, "String"),
            DT::FixedString(n) => write!(f, "FixedString({n})"),
            DT::UInt8 => write!(f, "UInt8"),
            DT::UInt16 => write!(f, "UInt16"),
            DT::UInt32 => write!(f, "UInt32"),
            DT::UInt64 => write!(f, "UInt64"),
            DT::UInt128 => write!(f, "UInt128"),
            DT::UInt256 => write!(f, "UInt256"),
            DT::Int8 => write!(f, "Int8"),
            DT::Int16 => write!(f, "Int16"),
            DT::Int32 => write!(f, "Int32"),
            DT::Int64 => write!(f, "Int64"),
            DT::Int128 => write!(f, "Int128"),
            DT::Int256 => write!(f, "Int256"),
            DT::Float32 => write!(f, "Float32"),
            DT::Float64 => write!(f, "Float64"),
            DT::Decimal { precision, scale } => write!(f, "Decimal({precision}, {scale})"),
            DT::Decimal32 { scale } => write!(f, "Decimal32({scale})"),
            DT::Decimal64 { scale } => write!(f, "Decimal64({scale})"),
            DT::Decimal128 { scale } => write!(f, "Decimal128({scale})"),
            DT::Decimal256 { scale } => write!(f, "Decimal256({scale})"),
            DT::Date => write!(f, "Date"),
            DT::Date32 => write!(f, "Date32"),
            DT::DateTime { timezone } => {
                write!(f, "DateTime")?;
                if let Some(tz) = timezone {
                    write!(f, "({tz})")?;
                }
                Ok(())
            }
            DT::DateTime64 {
                precision,
                timezone,
            } => {
                write!(f, "DateTime64({precision}")?;
                if let Some(tz) = timezone {
                    write!(f, ", {tz}")?;
                }
                write!(f, ")")
            }
            DT::Json => write!(f, "JSON"),
            DT::Uuid => write!(f, "UUID"),
            DT::IPv4 => write!(f, "IPv4"),
            DT::IPv6 => write!(f, "IPv6"),
            DT::LowCardinality(inner) => write!(f, "LowCardinality({inner})"),
            DT::Nested(elements) => {
                write!(f, "Nested(")?;
                let mut first = true;
                for (name, value) in elements {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name} {value}")?;
                }
                write!(f, ")")
            }
            DT::Array(inner) => write!(f, "Array({inner})"),
            DT::Map { key, value } => write!(f, "Map({key}, {value})"),
            DT::Tuple(elements) => {
                write!(f, "Tuple(")?;
                let mut first = true;
                for (name, t) in elements {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    if let Some(name) = name {
                        write!(f, "{name} ")?;
                    }
                    write!(f, "{t}")?;
                }
                write!(f, ")")
            }
            DT::Enum(variants) => {
                write!(f, "Enum(")?;
                let mut first = true;
                for (variant, id) in variants {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }

                    write!(f, "{variant}")?;

                    if let Some(id) = id {
                        write!(f, " = {id}")?;
                    }
                }
                write!(f, ")")
            }
            DT::SimpleAggregateFunction {
                function,
                arguments,
            } => {
                write!(f, "SimpleAggregateFunction({function}")?;
                for argument in arguments {
                    write!(f, ", {argument}")?;
                }
                write!(f, ")")
            }
            DT::AggregateFunction {
                function,
                arguments,
            } => {
                write!(f, "AggregateFunction({function}")?;
                for argument in arguments {
                    write!(f, ", {argument}")?;
                }
                write!(f, ")")
            }
            DT::Nothing => write!(f, "Nothing"),
            DT::CompoundIdentifier(identifiers) => {
                let mut first = true;
                for identifier in identifiers {
                    if first {
                        first = false;
                    } else {
                        write!(f, ".")?;
                    }

                    write!(f, "{identifier}")?;
                }
                Ok(())
            }
            DT::SingleIdentifier(identifier) => write!(f, "{identifier}"),
        }
    }
}

impl FromStr for ClickHouseDataType {
    type Err = peg::error::ParseError<peg::str::LineCol>;

    /// Attempt to create a ClickHouseDataType from a string representation of the type.
    /// May return a parse error if the type string is malformed, or if our implementation is out of date or incorrect
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        clickhouse_parser::data_type(s)
    }
}
