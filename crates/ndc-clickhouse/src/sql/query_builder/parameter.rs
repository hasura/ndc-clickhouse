use super::{format::escape_string, Expr, Parameter, Value};
use crate::sql::ast::format::display_separated;
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
        // note: serializing null as \N seems to be a default configuration
        // we may need to add a configuration option for this in the future,
        // but let's wait until a user actually asks for it
        // ref: https://clickhouse.com/docs/en/operations/settings/formats#format_tsv_null_representation
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
                    write!(f, "'{}':", escape_string(key))?;
                    print_parameter_value(f, value)
                })
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
            (json!("foo"), "foo"),
            (json!(true), "true"),
            (json!(false), "false"),
            (json!({ "foo": "bar"}), "{'foo':'bar'}"),
            (json!(["foo", "bar"]), "['foo','bar']"),
            (json!(null), "\\N"),
        ];

        for (value, printed) in test_cases {
            assert_eq!(ParameterValue(&value).to_string().as_str(), printed)
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
                "{'foo': ['bar'],'baz': NULL}",
            ),
        ];

        for (value, data_type_string, expected) in &test_cases {
            let data_type = ClickHouseDataType::from_str(data_type_string)
                .expect("Data type string should be valid ClickHouseDataType");
            let inlined = parameters.bind_json(value, data_type.into());

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
        ];
        let mut placeholders = vec![];

        for (value, data_type_string, _, _, _) in &test_cases {
            let data_type = ClickHouseDataType::from_str(data_type_string)
                .expect("Data type string should be valid ClickHouseDataType");
            let placeholder = parameters.bind_json(value, data_type.into());
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
