use crate::sql::ast::format::display_separated;

use super::{format::escape_string, Expr, Parameter, Value};
use common::clickhouse_parser::{datatype::ClickHouseDataType, parameterized_query::ParameterType};
use std::fmt::Display;

pub struct ParameterBuilder {
    inline_parameters: bool,
    index: u32,
    parameters: Vec<(String, String)>,
}

impl ParameterBuilder {
    pub fn new(inline_parameters: bool) -> Self {
        Self {
            inline_parameters,
            index: 0,
            parameters: vec![],
        }
    }
    fn bind_parameter(&mut self, value: String, data_type: ParameterType) -> Expr {
        let parameter_name = format!("param_p{}", self.index);
        let placeholder_name = format!("p{}", self.index);
        self.index += 1;

        self.parameters.push((parameter_name, value));

        let placeholder = Parameter::new(placeholder_name, data_type);
        placeholder.into_expr()
    }
    pub fn bind_json(&mut self, value: &serde_json::Value, data_type: ParameterType) -> Expr {
        if self.inline_parameters {
            let value: Value = value.into();
            value.into_expr()
        } else {
            self.bind_parameter(ParameterValue(value).to_string(), data_type)
        }
    }
    pub fn bind_string(&mut self, value: &str) -> Expr {
        if self.inline_parameters {
            let value = Value::SingleQuotedString(value.to_owned());
            value.into_expr()
        } else {
            self.bind_parameter(
                escape_string(value).to_string(),
                ClickHouseDataType::String.into(),
            )
        }
    }
    pub fn into_parameters(self) -> Vec<(String, String)> {
        self.parameters
    }
}

struct ParameterValue<'a>(&'a serde_json::Value);

impl<'a> Display for ParameterValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            // top level string should not be quoted
            serde_json::Value::String(s) => write!(f, "{}", escape_string(s)),
            _ => print_parameter_value(f, self.0),
        }
    }
}

fn print_parameter_value(
    f: &mut std::fmt::Formatter<'_>,
    value: &serde_json::Value,
) -> std::fmt::Result {
    match value {
        serde_json::Value::Null => write!(f, "\\N"),
        serde_json::Value::Bool(b) => {
            if *b {
                write!(f, "true")
            } else {
                write!(f, "false")
            }
        }
        serde_json::Value::Number(n) => write!(f, "{n}"),
        serde_json::Value::String(s) => write!(f, "'{}'", escape_string(s)),
        serde_json::Value::Array(arr) => {
            write!(
                f,
                "[{}]",
                display_separated(arr, ",", |f, i| print_parameter_value(f, i))
            )
        }
        serde_json::Value::Object(obj) => {
            write!(
                f,
                "{{{}}}",
                display_separated(obj, ",", |f, (key, value)| {
                    write!(f, "''{}':", escape_string(key))?;
                    print_parameter_value(f, value)
                })
            )
        }
    }
}
