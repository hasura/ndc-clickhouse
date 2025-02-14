use crate::clickhouse_parser::datatype::ClickHouseDataType;
use ndc_models::{ArgumentName, CollectionName, FieldName};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
/// the main configuration file
pub struct ServerConfigFile {
    #[serde(rename = "$schema")]
    pub schema: String,
    /// A list of tables available in this database
    ///
    /// The map key is a unique table alias that defaults to defaults to "<table_schema>_<table_name>",
    /// except for tables in the "default" schema where the table name is used
    /// This is the name exposed to the engine, and may be configured by users.
    /// When the configuration is updated, the table is identified by name and schema, and changes to the alias are preserved.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub tables: BTreeMap<CollectionName, TableConfigFile>,
    /// Optionally define custom parameterized queries here
    /// Note the names must not match table names
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub queries: BTreeMap<CollectionName, ParameterizedQueryConfigFile>,
}

impl Default for ServerConfigFile {
    fn default() -> Self {
        Self {
            schema: CONFIG_SCHEMA_FILE_NAME.to_string(),
            tables: Default::default(),
            queries: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TableConfigFile {
    /// The table name
    pub name: String,
    /// The table schema
    pub schema: String,
    /// Comments are sourced from the database table comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<PrimaryKey>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub arguments: BTreeMap<ArgumentName, String>,
    /// The map key is a column alias identifying the table and may be customized.
    /// It defaults to the table name.
    /// When the configuration is updated, the column is identified by name, and changes to the alias are preserved.
    pub return_type: ReturnType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PrimaryKey {
    pub name: String,
    /// The names of columns in this primary key
    pub columns: Vec<FieldName>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReturnType {
    /// A custom return type definition
    /// The keys are column names, the values are parsable clichouse datatypes
    Definition {
        columns: BTreeMap<FieldName, MaybeClickhouseDataType>,
    },
    /// the same as the return type for another table
    TableReference {
        /// the table alias must match a key in `tables`, and the query must return the same type as that table
        /// alternatively, the alias may reference another parameterized query which has a return type definition,
        table_name: CollectionName,
    },
    /// The same as the return type for another query
    QueryReference {
        /// the table alias must match a key in `tables`, and the query must return the same type as that table
        /// alternatively, the alias may reference another parameterized query which has a return type definition,
        query_name: CollectionName,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum MaybeClickhouseDataType {
    Valid(ClickHouseDataType),
    Invalid(String),
}

// JSONSchema for MaybeClickhouseDataType is String
impl JsonSchema for MaybeClickhouseDataType {
    fn schema_name() -> String {
        String::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }

    fn is_referenceable() -> bool {
        String::is_referenceable()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        String::schema_id()
    }
}

impl Default for ReturnType {
    fn default() -> Self {
        Self::Definition {
            columns: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ParameterizedQueryConfigFile {
    /// Whether this query should be exposed as a procedure (mutating) or collection (non-mutating)
    pub exposed_as: ParameterizedQueryExposedAs,
    /// A comment that will be exposed in the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// A relative path to a sql file
    pub file: String,
    /// Either a type definition for the return type for this query,
    /// or a reference to another return type: either a table's alias,
    /// or another query's alias. If another query, that query must have a return type definition.
    pub return_type: ReturnType,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParameterizedQueryExposedAs {
    #[default]
    Collection,
    Procedure,
}

pub const CONFIG_FILE_NAME: &str = "configuration.json";
pub const CONFIG_SCHEMA_FILE_NAME: &str = "configuration.schema.json";
