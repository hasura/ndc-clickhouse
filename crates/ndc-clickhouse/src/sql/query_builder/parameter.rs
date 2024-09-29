use super::{format::escape_string, Expr, Parameter, QueryBuilderError, Value};
use crate::sql::ast::format::display_separated;
use common::clickhouse_parser::{datatype::ClickHouseDataType, parameterized_query::ParameterType};
use std::fmt::Display;

pub struct ParameterBuilder {
    inline_parameters: bool,
    index: u32,
    parameters: Vec<(String, ParameterValue)>,
}

impl ParameterBuilder {
    pub fn new(inline_parameters: bool) -> Self {
        Self {
            inline_parameters,
            index: 0,
            parameters: vec![],
        }
    }
    fn bind_parameter(&mut self, value: ParameterValue, data_type: ParameterType) -> Expr {
        let parameter_name = format!("param_p{}", self.index);
        let placeholder_name = format!("p{}", self.index);
        self.index += 1;

        self.parameters.push((parameter_name, value));

        let placeholder = Parameter::new(placeholder_name, data_type);
        placeholder.into_expr()
    }
    pub fn bind_json(
        &mut self,
        value: &serde_json::Value,
        data_type: ParameterType,
    ) -> Result<Expr, QueryBuilderError> {
        let value = match &data_type {
            ParameterType::DataType(data_type) => Value::try_from_json(value, data_type)?,
            ParameterType::Identifier => match value {
                serde_json::Value::String(s) => Value::SingleQuotedString(s.to_owned()),
                _ => {
                    return Err(QueryBuilderError::UnsupportedParameterCast {
                        value: value.to_owned(),
                        data_type: data_type.to_owned(),
                    })
                }
            },
        };

        if self.inline_parameters {
            Ok(value.into_expr())
        } else {
            Ok(self.bind_parameter(ParameterValue(value), data_type))
        }
    }
    pub fn bind_string(&mut self, value: &str) -> Expr {
        if self.inline_parameters {
            let value = Value::SingleQuotedString(value.to_owned());
            value.into_expr()
        } else {
            self.bind_parameter(
                ParameterValue(Value::SingleQuotedString(value.to_owned())),
                ClickHouseDataType::String.into(),
            )
        }
    }
    pub fn into_parameters(self) -> Vec<(String, String)> {
        self.parameters
            .into_iter()
            .map(|(name, value)| (name, value.to_string()))
            .collect()
    }
}

struct ParameterValue(Value);

impl Display for ParameterValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            // top level string should not be quoted
            Value::SingleQuotedString(s) => write!(f, "{}", escape_string(s)),
            _ => print_parameter_value(f, &self.0),
        }
    }
}

fn print_parameter_value(f: &mut std::fmt::Formatter<'_>, value: &Value) -> std::fmt::Result {
    match value {
        // note: serializing null as \N seems to be a default configuration
        // we may need to add a configuration option for this in the future,
        // but let's wait until a user actually asks for it
        // ref: https://clickhouse.com/docs/en/operations/settings/formats#format_tsv_null_representation
        Value::Null => write!(f, "\\N"),
        Value::Boolean(b) => {
            if *b {
                write!(f, "true")
            } else {
                write!(f, "false")
            }
        }
        Value::Number(n) => write!(f, "{n}"),
        Value::SingleQuotedString(s) => write!(f, "'{}'", escape_string(s)),
        Value::Array(arr) => {
            write!(
                f,
                "[{}]",
                display_separated(arr, ",", |f, i| print_parameter_value(f, i))
            )
        }
        Value::Map(elements) => {
            write!(
                f,
                "{{{}}}",
                display_separated(elements, ",", |f, (key, value)| {
                    write!(f, "'{}':", escape_string(key))?;
                    print_parameter_value(f, value)
                })
            )
        }
        Value::Tuple(elements) => {
            write!(
                f,
                "({})",
                display_separated(elements, ",", |f, i| print_parameter_value(f, i))
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn can_print_parameter() {
        let test_cases = vec![
            (json!("foo"), "String", "foo"),
            (json!(true), "Bool", "true"),
            (json!(false), "Bool", "false"),
            (
                json!({ "foo": "bar"}),
                "Map(String, String)",
                "{'foo':'bar'}",
            ),
            (json!(["foo", "bar"]), "Array(String)", "['foo','bar']"),
            (json!(null), "Nullable(String)", "\\N"),
        ];

        for (value, data_type, printed) in test_cases {
            let data_type =
                ClickHouseDataType::from_str(data_type).expect("Should parse data type");

            let value = Value::try_from_json(&value, &data_type)
                .expect("Should convert type based on data type");

            assert_eq!(ParameterValue(value).to_string().as_str(), printed)
        }
    }
    #[test]
    fn inline_string_parameters() {
        let mut parameters = ParameterBuilder::new(true);

        let test_cases = vec![
            ("foo", "'foo'"),
            ("foo'bar", "'foo\\'bar'"),
            ("foo\nbar", "'foo\\nbar'"),
            ("foo\n\\\r\tbar", "'foo\\n\\\\\\r\\tbar'"),
        ];

        for (value, expected) in &test_cases {
            let inlined = parameters.bind_string(value);

            assert_eq!(
                &inlined.to_string().as_str(),
                expected,
                "string parameter {} should be inlined as {}",
                value,
                expected
            )
        }

        let bound_parameters = parameters.into_parameters();
        assert_eq!(
            bound_parameters,
            vec![],
            "bound params should be empty after inlining parameters"
        );
    }
    #[test]
    fn inline_json_parameters() {
        let mut parameters = ParameterBuilder::new(true);

        let test_cases = vec![
            (json!("foo"), "String", "'foo'"),
            (json!(["foo", "bar"]), "Array(String)", "['foo', 'bar']"),
            (
                json!({"foo": "bar"}),
                "Map(String, String)",
                "{'foo': 'bar'}",
            ),
            (json!(null), "Nullable(String)", "NULL"),
            (json!(2), "Int32", "2"),
            (json!(true), "Bool", "TRUE"),
            (json!(false), "Bool", "FALSE"),
            (
                json!({ "foo": ["bar"], "baz": null }),
                "Tuple(foo Array(String), baz Nullable(String))",
                "(['bar'], NULL)",
            ),
            (json!(["foo", 1]), "Tuple(String, Int32)", "('foo', 1)"),
        ];

        for (value, data_type_string, expected) in &test_cases {
            let data_type = ClickHouseDataType::from_str(data_type_string)
                .expect("Data type string should be valid ClickHouseDataType");
            let inlined = parameters
                .bind_json(value, data_type.into())
                .expect("Should succesessfully bind parameter");

            assert_eq!(
                &inlined.to_string().as_str(),
                expected,
                "parameter {} of type {} should be inlined as {}",
                value,
                data_type_string,
                expected
            )
        }

        let bound_parameters = parameters.into_parameters();
        assert_eq!(
            bound_parameters,
            vec![],
            "bound params should be empty after inlining parameters"
        );
    }

    #[test]
    fn bind_string_parameters() {
        let mut parameters = ParameterBuilder::new(false);

        let test_cases = vec![
            ("foo", "{p0:String}", "param_p0", "foo"),
            ("foo\rbar", "{p1:String}", "param_p1", "foo\\rbar"),
            ("foo'bar", "{p2:String}", "param_p2", "foo\\'bar"),
            (
                "foo\\\r\t\nbar",
                "{p3:String}",
                "param_p3",
                "foo\\\\\\r\\t\\nbar",
            ),
        ];
        let mut placeholders = vec![];

        for (value, _, _, _) in &test_cases {
            let placeholder = parameters.bind_string(value);
            placeholders.push(placeholder);
        }

        let values = parameters.into_parameters();

        for (
            (
                (_value, expected_placeholder, expected_param_name, expected_param_value),
                placeholder,
            ),
            (param_name, param_value),
        ) in test_cases
            .iter()
            .zip(placeholders.iter())
            .zip(values.iter())
        {
            assert_eq!(expected_placeholder, &placeholder.to_string().as_str());
            assert_eq!(expected_param_name, param_name,);
            assert_eq!(expected_param_value, param_value);
        }
    }
    #[test]
    fn bind_json_parameters() {
        let mut parameters = ParameterBuilder::new(false);

        let test_cases = vec![
            (json!("foo"), "String", "{p0:String}", "param_p0", "foo"),
            (
                json!("foo\rbar"),
                "String",
                "{p1:String}",
                "param_p1",
                "foo\\rbar",
            ),
            (
                json!("foo'bar"),
                "String",
                "{p2:String}",
                "param_p2",
                "foo\\'bar",
            ),
            (
                json!("foo\\\r\t\nbar"),
                "String",
                "{p3:String}",
                "param_p3",
                "foo\\\\\\r\\t\\nbar",
            ),
            (json!(1), "UInt32", "{p4:UInt32}", "param_p4", "1"),
            (
                json!(null),
                "Nullable(String)",
                "{p5:Nullable(String)}",
                "param_p5",
                "\\N",
            ),
            (json!(true), "Bool", "{p6:Bool}", "param_p6", "true"),
            (json!(false), "Bool", "{p7:Bool}", "param_p7", "false"),
            (
                json!({"foo": "bar"}),
                "Map(String, String)",
                "{p8:Map(String, String)}",
                "param_p8",
                "{'foo':'bar'}",
            ),
            (
                json!({"foo'\n\r\t\\bar": "baz"}),
                "Map(String, String)",
                "{p9:Map(String, String)}",
                "param_p9",
                "{'foo\\'\\n\\r\\t\\\\bar':'baz'}",
            ),
            (
                json!(["foo", "bar"]),
                "Array(String)",
                "{p10:Array(String)}",
                "param_p10",
                "['foo','bar']",
            ),
            (
                json!(["foo", "bar"]),
                "Tuple(String, String)",
                "{p11:Tuple(String, String)}",
                "param_p11",
                "('foo','bar')",
            ),
            (
                json!({"foo": "bar", "baz": "qux"}),
                "Tuple(foo String, baz String)",
                "{p12:Tuple(foo String, baz String)}",
                "param_p12",
                "('bar','qux')",
            ),
        ];
        let mut placeholders = vec![];

        for (value, data_type_string, _, _, _) in &test_cases {
            let data_type = ClickHouseDataType::from_str(data_type_string)
                .expect("Data type string should be valid ClickHouseDataType");
            let placeholder = parameters
                .bind_json(value, data_type.into())
                .expect("Should successfully bind parameter");
            placeholders.push(placeholder);
        }

        let values = parameters.into_parameters();

        for (
            (
                (
                    _value,
                    _data_type,
                    expected_placeholder,
                    expected_param_name,
                    expected_param_value,
                ),
                placeholder,
            ),
            (param_name, param_value),
        ) in test_cases
            .iter()
            .zip(placeholders.iter())
            .zip(values.iter())
        {
            assert_eq!(expected_placeholder, &placeholder.to_string().as_str());
            assert_eq!(expected_param_name, param_name,);
            assert_eq!(expected_param_value, param_value);
        }
    }
}
