use std::{fmt::Display, str::FromStr};

use super::{
    clickhouse_parser,
    datatype::{ClickHouseDataType, Identifier},
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParameterizedQuery {
    pub elements: Vec<ParameterizedQueryElement>,
}

impl Display for ParameterizedQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for element in &self.elements {
            write!(f, "{}", element)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: Identifier,
    pub r#type: ParameterType,
}

impl Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}: {}}}", self.name, self.r#type)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterType {
    DataType(ClickHouseDataType),
    Identifier,
}

impl From<ClickHouseDataType> for ParameterType {
    fn from(value: ClickHouseDataType) -> Self {
        Self::DataType(value)
    }
}

impl Display for ParameterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterType::DataType(t) => write!(f, "{t}"),
            ParameterType::Identifier => write!(f, "Identifier"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterizedQueryElement {
    String(String),
    Parameter(Parameter),
}

impl Display for ParameterizedQueryElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterizedQueryElement::String(s) => write!(f, "{s}"),
            ParameterizedQueryElement::Parameter(p) => write!(f, "{p}"),
        }
    }
}

impl FromStr for ParameterizedQuery {
    type Err = peg::error::ParseError<peg::str::LineCol>;

    /// Attempt to create a ParameterizedQuery from a string.
    /// May return a parse error if the type string is malformed, or if our implementation is out of date or incorrect
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        clickhouse_parser::parameterized_query(s)
    }
}
