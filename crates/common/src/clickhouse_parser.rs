pub mod datatype;
pub mod parameterized_query;
use self::datatype::{
    AggregateFunctionDefinition, AggregateFunctionParameter, ClickHouseDataType as DT, Identifier,
    SingleQuotedString,
};

use self::parameterized_query::{
    Parameter, ParameterType, ParameterizedQuery, ParameterizedQueryElement,
};

peg::parser! {
  grammar clickhouse_parser() for str {
    pub rule parameterized_query() -> ParameterizedQuery = elements:parameterized_query_element()* statement_end()? { ParameterizedQuery { elements } }
    // single quoted strings, or anything that doesn't match a statement end or a parameter, is a part of the string
    // this prevents matching on parameters insides of single quotes, if for some reason we have sql like that?
    rule parameterized_query_element() -> ParameterizedQueryElement = s:$((single_quoted_string_value() / !parameter() !statement_end() [_])+) { ParameterizedQueryElement::String(s.to_string()) } / p:parameter() { ParameterizedQueryElement::Parameter(p)}
    rule parameter() -> Parameter = p:("{" _ name:identifier() _ ":" _ t:parameter_type() _ "}" { Parameter { name, r#type: t }})
    rule parameter_type() -> ParameterType = d:data_type() { ParameterType::DataType(d) } / "Identifier" { ParameterType::Identifier }
    rule statement_end() =  _ ";" _

    pub rule data_type() -> DT = nullable()
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
        / uuid()
        / ipv4()
        / ipv6()
        / low_cardinality()
        / nested()
        / array()
        / map()
        / tuple()
        / r#enum()
        / aggregate_function()
        / simple_aggregate_function()
        / nothing()
    rule nullable() -> DT = "Nullable(" t:data_type() ")" { DT::Nullable(Box::new(t)) }
    rule uint8() -> DT = "UInt8" { DT::UInt8 }
    rule uint16() -> DT = "UInt16" { DT::UInt16 }
    rule uint32() -> DT = "UInt32" { DT::UInt32 }
    rule uint64() -> DT = "UInt64" { DT::UInt64 }
    rule uint128() -> DT = "UInt128" { DT::UInt128 }
    rule uint256() -> DT = "UInt256" { DT::UInt256 }
    rule int8() -> DT = "Int8" { DT::Int8 }
    rule int16() -> DT = "Int16" { DT::Int16 }
    rule int32() -> DT = "Int32" { DT::Int32 }
    rule int64() -> DT = "Int64" { DT::Int64 }
    rule int128() -> DT = "Int128" { DT::Int128 }
    rule int256() -> DT = "Int256" { DT::Int256 }
    rule float32() -> DT = "Float32" { DT::Float32 }
    rule float64() -> DT = "Float64" { DT::Float64 }
    rule decimal() -> DT = "Decimal(" precision:integer_value() comma_separator() scale:integer_value() ")" { DT::Decimal { precision, scale }  }
    rule decimal32() -> DT = "Decimal32(" scale:integer_value() ")" { DT::Decimal32 { scale } }
    rule decimal64() -> DT = "Decimal64(" scale:integer_value() ")" { DT::Decimal64 { scale } }
    rule decimal128() -> DT = "Decimal128(" scale:integer_value() ")" { DT::Decimal128 { scale } }
    rule decimal256() -> DT = "Decimal256(" scale:integer_value() ")" { DT::Decimal256 { scale } }
    rule bool() -> DT = "Bool" { DT::Bool }
    rule string() -> DT = "String" { DT::String }
    rule fixed_string() -> DT = "FixedString(" n:integer_value() ")" { DT::FixedString(n) }
    rule date() -> DT = "Date" { DT::Date }
    rule date32() -> DT = "Date32" { DT::Date32 }
    rule date_time() -> DT = "DateTime" tz:("(" tz:single_quoted_string_value()? ")" { tz })? { DT::DateTime { timezone: tz.flatten().map(|s| s.to_owned()) } }
    rule date_time64() -> DT = "DateTime64(" precision:integer_value() tz:(comma_separator() tz:single_quoted_string_value()? { tz })? ")" { DT::DateTime64{ precision, timezone: tz.flatten().map(|s| s.to_owned())} }
    rule uuid() -> DT = "UUID" { DT::Uuid }
    rule ipv4() -> DT = "IPv4" { DT::IPv4 }
    rule ipv6() -> DT = "IPv6" { DT::IPv6 }
    rule low_cardinality() -> DT = "LowCardinality(" t:data_type() ")" { DT::LowCardinality(Box::new(t)) }
    rule nested() -> DT =  "Nested(" e:((n:identifier() __ t:data_type() { (n, t)}) ** comma_separator()) ")" { DT::Nested(e) }
    rule array() -> DT =  "Array(" t:data_type() ")" { DT::Array(Box::new(t)) }
    rule map() -> DT =  "Map(" k:data_type() comma_separator()  v:data_type() ")" { DT::Map { key: Box::new(k), value: Box::new(v) } }
    rule tuple() -> DT =  "Tuple(" e:((n:(n:identifier() __ { n })? t:data_type() { (n, t) }) ** comma_separator())  ")" { DT::Tuple(e) }
    rule r#enum() -> DT = "Enum" ("8" / "16")?  "(" e:((n:single_quoted_string_value() i:(_ "=" _ i:integer_value() { i })? { (n, i) }) ** comma_separator()) ")" { DT::Enum(e)}
    rule aggregate_function() -> DT = "AggregateFunction(" f:aggregate_function_definition() comma_separator() a:(data_type() ** comma_separator()) ")" { DT::AggregateFunction { function: f, arguments:  a }}
    rule simple_aggregate_function() -> DT =  "SimpleAggregateFunction(" f:aggregate_function_definition() comma_separator() a:(data_type() ** comma_separator()) ")" { DT::SimpleAggregateFunction { function: f, arguments:  a }}
    rule nothing() -> DT = "Nothing" { DT::Nothing }

    rule aggregate_function_definition() -> AggregateFunctionDefinition = n:identifier() p:("(" p:(aggregate_function_parameter() ** comma_separator()) ")" { p })? { AggregateFunctionDefinition { name: n, parameters: p }}
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

    /// One or more whitespace
    rule __ = [' ' | '\t' | '\r' | '\n']+
    /// Optional whitespace
    rule _ = [' ' | '\t' | '\r' | '\n']*
    /// A comma surrounded by optional whitespace
    rule comma_separator() = _ "," _
  }
}

#[test]
fn can_parse_clickhouse_data_type() {
    let data_types = vec![
        ("Int32", DT::Int32),
        ("Nullable(Int32)", DT::Nullable(Box::new(DT::Int32))),
        ("Nullable(Date32)", DT::Nullable(Box::new(DT::Date32))),
        (
            "DateTime64(9)",
            DT::DateTime64 {
                precision: 9,
                timezone: None,
            },
        ),
        ("Float64", DT::Float64),
        ("Date", DT::Date),
        (
            "DateTime('Asia/Istanbul\\\\')",
            DT::DateTime {
                timezone: Some(SingleQuotedString("Asia/Istanbul\\\\".to_string())),
            },
        ),
        (
            "LowCardinality(String)",
            DT::LowCardinality(Box::new(DT::String)),
        ),
        (
            "Map(LowCardinality(String), String)",
            DT::Map {
                key: Box::new(DT::LowCardinality(Box::new(DT::String))),
                value: Box::new(DT::String),
            },
        ),
        (
            "Array(DateTime64(9))",
            DT::Array(Box::new(DT::DateTime64 {
                precision: 9,
                timezone: None,
            })),
        ),
        (
            "Array(Map(LowCardinality(String), String))",
            DT::Array(Box::new(DT::Map {
                key: Box::new(DT::LowCardinality(Box::new(DT::String))),
                value: Box::new(DT::String),
            })),
        ),
        (
            "Tuple(String, Int32)",
            DT::Tuple(vec![(None, DT::String), (None, DT::Int32)]),
        ),
        (
            "Tuple(n String, \"i\" Int32, `u` UInt8)",
            DT::Tuple(vec![
                (Some(Identifier::Unquoted("n".to_string())), DT::String),
                (Some(Identifier::DoubleQuoted("i".to_string())), DT::Int32),
                (Some(Identifier::BacktickQuoted("u".to_string())), DT::UInt8),
            ]),
        ),
        (
            "SimpleAggregateFunction(sum, UInt64)",
            DT::SimpleAggregateFunction {
                function: AggregateFunctionDefinition {
                    name: Identifier::Unquoted("sum".to_string()),
                    parameters: None,
                },
                arguments: vec![DT::UInt64],
            },
        ),
    ];

    for (s, t) in data_types {
        let parsed = clickhouse_parser::data_type(s);
        assert_eq!(parsed, Ok(t), "Able to parse correctly");
    }
}

#[test]
fn can_parse_parameterized_query() {
    let query = r#"
    SELECT Name
    FROM "default"."Artist"
    WHERE ArtistId = {ArtistId:Int32} AND Name != {ArtistName: String};

"#;
    let expected = ParameterizedQuery {
        elements: vec![
            ParameterizedQueryElement::String(
                "\n    SELECT Name\n    FROM \"default\".\"Artist\"\n    WHERE ArtistId = "
                    .to_string(),
            ),
            ParameterizedQueryElement::Parameter(Parameter {
                name: Identifier::Unquoted("ArtistId".to_string()),
                r#type: ParameterType::DataType(DT::Int32),
            }),
            ParameterizedQueryElement::String(" AND Name != ".to_string()),
            ParameterizedQueryElement::Parameter(Parameter {
                name: Identifier::Unquoted("ArtistName".to_string()),
                r#type: ParameterType::DataType(DT::String),
            }),
        ],
    };
    let parsed = clickhouse_parser::parameterized_query(query);
    assert_eq!(parsed, Ok(expected), "can parse parameterized query");
}

#[test]
fn can_parse_empty_parameterized_query() {
    let query = r#""#;
    let expected = ParameterizedQuery { elements: vec![] };
    let parsed = clickhouse_parser::parameterized_query(query);
    assert_eq!(parsed, Ok(expected), "can parse parameterized query");
}

#[test]
fn does_not_parse_parameters_insides_quoted_strings() {
    let query = r#"
    SELECT Name
    FROM "default"."Artist"
    WHERE ArtistId = {ArtistId:Int32} AND Name = '{ArtistName: String}';

"#;
    let expected = ParameterizedQuery {
        elements: vec![
            ParameterizedQueryElement::String(
                "\n    SELECT Name\n    FROM \"default\".\"Artist\"\n    WHERE ArtistId = "
                    .to_string(),
            ),
            ParameterizedQueryElement::Parameter(Parameter {
                name: Identifier::Unquoted("ArtistId".to_string()),
                r#type: ParameterType::DataType(DT::Int32),
            }),
            ParameterizedQueryElement::String(" AND Name = '{ArtistName: String}'".to_string()),
        ],
    };
    let parsed = clickhouse_parser::parameterized_query(query);
    assert_eq!(parsed, Ok(expected), "can parse parameterized query");
}
