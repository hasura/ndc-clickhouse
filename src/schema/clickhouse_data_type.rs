use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingleQuotedString(String);

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ClickhouseDataType {
    Nullable(Box<ClickhouseDataType>),
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
    LowCardinality(Box<ClickhouseDataType>),
    Nested(Vec<(Identifier, ClickhouseDataType)>),
    Array(Box<ClickhouseDataType>),
    Map {
        key: Box<ClickhouseDataType>,
        value: Box<ClickhouseDataType>,
    },
    Tuple(Vec<(Option<Identifier>, ClickhouseDataType)>),
    Enum(Vec<(SingleQuotedString, Option<u32>)>),
    SimpleAggregateFunction {
        function: AggregateFunctionDefinition,
        arguments: Vec<ClickhouseDataType>,
    },
    AggregateFunction {
        function: AggregateFunctionDefinition,
        arguments: Vec<ClickhouseDataType>,
    },
    Nothing,
}

impl Display for ClickhouseDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ClickhouseDataType as DT;
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
        }
    }
}

impl FromStr for ClickhouseDataType {
    type Err = peg::error::ParseError<peg::str::LineCol>;

    /// Attempt to create a ClickhouseDataType from a string representation of the type.
    /// May return a parse error if the type string is malformed, or if our implementation is out of date or incorrect
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        clickhouse_parser::data_type(s)
    }
}

peg::parser! {
  grammar clickhouse_parser() for str {
    use ClickhouseDataType as CDT;
    pub rule data_type() -> ClickhouseDataType = nullable()
        / uint256()
        / uint128()
        / uint64()
        / uint32()
        / uint16()
        / uint8()
        / int256()
        / int128()
        / int64()
        / int32()
        / int16()
        / int8()
        / float32()
        / float64()
        / decimal256()
        / decimal128()
        / decimal64()
        / decimal32()
        / decimal()
        / bool()
        / string()
        / fixed_string()
        / date_time64()
        / date_time()
        / date32()
        / date()
        / json()
        / uuid()
        / ipv4()
        / ipv6()
        / low_cardinality()
        / nested()
        / array()
        / map()
        / tuple()
        / r#enum()
        / nothing()
    rule nullable() -> ClickhouseDataType = "Nullable(" t:data_type() ")" { CDT::Nullable(Box::new(t)) }
    rule uint8() -> ClickhouseDataType = "UInt8" { CDT::UInt8 }
    rule uint16() -> ClickhouseDataType = "UInt16" { CDT::UInt16 }
    rule uint32() -> ClickhouseDataType = "UInt32" { CDT::UInt32 }
    rule uint64() -> ClickhouseDataType = "UInt64" { CDT::UInt64 }
    rule uint128() -> ClickhouseDataType = "UInt128" { CDT::UInt128 }
    rule uint256() -> ClickhouseDataType = "UInt256" { CDT::UInt256 }
    rule int8() -> ClickhouseDataType = "Int8" { CDT::Int8 }
    rule int16() -> ClickhouseDataType = "Int16" { CDT::Int16 }
    rule int32() -> ClickhouseDataType = "Int32" { CDT::Int32 }
    rule int64() -> ClickhouseDataType = "Int64" { CDT::Int64 }
    rule int128() -> ClickhouseDataType = "Int128" { CDT::Int128 }
    rule int256() -> ClickhouseDataType = "Int256" { CDT::Int256 }
    rule float32() -> ClickhouseDataType = "Float32" { CDT::Float32 }
    rule float64() -> ClickhouseDataType = "Float64" { CDT::Float64 }
    rule decimal() -> ClickhouseDataType = "Decimal(" precision:integer_value() ", " scale:integer_value() ")" { CDT::Decimal { precision, scale }  }
    rule decimal32() -> ClickhouseDataType = "Decimal32(" scale:integer_value() ")" { CDT::Decimal32 { scale } }
    rule decimal64() -> ClickhouseDataType = "Decimal64(" scale:integer_value() ")" { CDT::Decimal64 { scale } }
    rule decimal128() -> ClickhouseDataType = "Decimal128(" scale:integer_value() ")" { CDT::Decimal128 { scale } }
    rule decimal256() -> ClickhouseDataType = "Decimal256(" scale:integer_value() ")" { CDT::Decimal256 { scale } }
    rule bool() -> ClickhouseDataType = "Bool" { CDT::Bool }
    rule string() -> ClickhouseDataType = "String" { CDT::String }
    rule fixed_string() -> ClickhouseDataType = "FixedString(" n:integer_value() ")" { CDT::FixedString(n) }
    rule date() -> ClickhouseDataType = "Date" { CDT::Date }
    rule date32() -> ClickhouseDataType = "Date32" { CDT::Date32 }
    rule date_time() -> ClickhouseDataType = "DateTime" tz:("(" tz:single_quoted_string_value()? ")" { tz })? { CDT::DateTime { timezone: tz.flatten().map(|s| s.to_owned()) } }
    rule date_time64() -> ClickhouseDataType = "DateTime64(" precision:integer_value() tz:(", " tz:single_quoted_string_value()? { tz })? ")" { CDT::DateTime64{ precision, timezone: tz.flatten().map(|s| s.to_owned())} }
    rule json() -> ClickhouseDataType = "JSON" { CDT::Json }
    rule uuid() -> ClickhouseDataType = "UUID" { CDT::Uuid }
    rule ipv4() -> ClickhouseDataType = "IPv4" { CDT::IPv4 }
    rule ipv6() -> ClickhouseDataType = "IPv6" { CDT::IPv6 }
    rule low_cardinality() -> ClickhouseDataType = "LowCardinality(" t:data_type() ")" { CDT::LowCardinality(Box::new(t)) }
    rule nested() -> ClickhouseDataType =  "Nested(" e:(("\""? n:identifier() "\""? " " t:data_type() { (n, t)}) ** ", ") ")" { CDT::Nested(e) }
    rule array() -> ClickhouseDataType =  "Array(" t:data_type() ")" { CDT::Array(Box::new(t)) }
    rule map() -> ClickhouseDataType =  "Map(" k:data_type() ", " v:data_type() ")" { CDT::Map { key: Box::new(k), value: Box::new(v) } }
    rule tuple() -> ClickhouseDataType =  "Tuple(" e:((n:(n:identifier() " " { n })? t:data_type() { (n, t) }) ** ", ")  ")" { CDT::Tuple(e) }
    rule r#enum() -> ClickhouseDataType = "Enum" ("8" / "16")?  "(" e:((n:single_quoted_string_value() i:(" = " i:integer_value() { i })? { (n, i) }) ** ", ") ")" { CDT::Enum(e)}
    rule aggregate_function() -> ClickhouseDataType = "AggregateFunction(" f:aggregate_function_definition() ", " a:(data_type() ** ", ") ")" { CDT::AggregateFunction { function: f, arguments:  a }}
    rule simple_aggregate_function() -> ClickhouseDataType =  "SimpleAggregateFunction(" f:aggregate_function_definition() ", " a:(data_type() ** ", ") ")" { CDT::SimpleAggregateFunction { function: f, arguments:  a }}
    rule nothing() -> ClickhouseDataType = "Nothing" { CDT::Nothing }



    rule aggregate_function_definition() -> AggregateFunctionDefinition = n:identifier() p:("(" p:(aggregate_function_parameter() ** ", ") ")" { p })? { AggregateFunctionDefinition { name: n, parameters: p }}
    rule aggregate_function_parameter() -> AggregateFunctionParameter = s:single_quoted_string_value() { AggregateFunctionParameter::SingleQuotedString(s)}
        / f:floating_point_value() { AggregateFunctionParameter::FloatingPoint(f)}
        / i:integer_value() { AggregateFunctionParameter::Integer(i) }
    rule floating_point_value() -> f64 = f:$(['0'..='9']+("." ['0'..='9']+)?) {? f.parse().or(Err("f64")) }
    rule integer_value() -> u32 = n:$(['0'..='9']+) {? n.parse().or(Err("u32")) }
    // parsing quoted strings
    // characters in quotes can be any char except quote char or backslash
    // unless the backslash is followed by any another character (and is thus not escaping our end quote)
    // for single quoted strings, single quotes in identifiers may also be escaped by another single quote, so include pairs of single quotes
    rule single_quoted_string_value() -> SingleQuotedString = "'" s:$(([^'\'' | '\\'] / "\\" [_] / "''")*) "'" { SingleQuotedString(s.to_string()) }
    rule double_quoted_string_value() -> Identifier = "\"" s:$(([^'\"' | '\\'] / "\\" [_])*) "\"" { Identifier::DoubleQuoted(s.to_string()) }
    rule backtick_quoted_string_value() -> Identifier = "`" s:$(([^'`' | '\\'] / "\\" [_])*) "`" { Identifier::BacktickQuoted(s.to_string()) }
    rule unquoted_identifier() -> Identifier = s:$(['a'..='z' | 'A'..='Z' | '_']['0'..='9' | 'a'..='z' | 'A'..='Z' | '_']*) { Identifier::Unquoted(s.to_string()) }
    rule identifier() -> Identifier = unquoted_identifier() / double_quoted_string_value() / backtick_quoted_string_value()
  }
}

#[test]
fn can_parse_clickhouse_data_type() {
    use ClickhouseDataType as CDT;
    let data_types = vec![
        ("Int32", CDT::Int32),
        ("Nullable(Int32)", CDT::Nullable(Box::new(CDT::Int32))),
        ("Nullable(Date32)", CDT::Nullable(Box::new(CDT::Date32))),
        (
            "DateTime64(9)",
            CDT::DateTime64 {
                precision: 9,
                timezone: None,
            },
        ),
        ("Float64", CDT::Float64),
        ("Date", CDT::Date),
        (
            "DateTime('Asia/Istanbul\\\\')",
            CDT::DateTime {
                timezone: Some(SingleQuotedString("Asia/Istanbul\\\\".to_string())),
            },
        ),
        (
            "LowCardinality(String)",
            CDT::LowCardinality(Box::new(CDT::String)),
        ),
        (
            "Map(LowCardinality(String), String)",
            CDT::Map {
                key: Box::new(CDT::LowCardinality(Box::new(CDT::String))),
                value: Box::new(CDT::String),
            },
        ),
        (
            "Array(DateTime64(9))",
            CDT::Array(Box::new(CDT::DateTime64 {
                precision: 9,
                timezone: None,
            })),
        ),
        (
            "Array(Map(LowCardinality(String), String))",
            CDT::Array(Box::new(CDT::Map {
                key: Box::new(CDT::LowCardinality(Box::new(CDT::String))),
                value: Box::new(CDT::String),
            })),
        ),
        (
            "Tuple(String, Int32)",
            CDT::Tuple(vec![(None, CDT::String), (None, CDT::Int32)]),
        ),
        (
            "Tuple(n String, \"i\" Int32, `u` UInt8)",
            CDT::Tuple(vec![
                (Some(Identifier::Unquoted("n".to_string())), CDT::String),
                (Some(Identifier::DoubleQuoted("i".to_string())), CDT::Int32),
                (
                    Some(Identifier::BacktickQuoted("u".to_string())),
                    CDT::UInt8,
                ),
            ]),
        ),
    ];

    for (s, t) in data_types {
        let parsed = clickhouse_parser::data_type(s);
        assert_eq!(parsed, Ok(t), "Able to parse correctly");
    }
}
