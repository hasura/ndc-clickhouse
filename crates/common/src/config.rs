use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::clickhouse_parser::{
    datatype::ClickHouseDataType,
    parameterized_query::{ParameterType, ParameterizedQuery},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfigFile {
    #[serde(rename = "$schema")]
    pub schema: String,
    /// A list of tables available in this database
    ///
    /// The map key is a unique table alias that defaults to defaults to "<table_schema>_<table_name>",
    /// except for tables in the "default" schema where the table name is used
    /// This is the name exposed to the engine, and may be configured by users.
    /// When the configuration is updated, the table is identified by name and schema, and changes to the alias are preserved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tables: Option<BTreeMap<String, TableConfig<String>>>,
    /// Optionally define custom parameterized queries here
    /// Note the names must not match table names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries: Option<BTreeMap<String, ParameterizedQueryConfigFile>>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// the connection part of the config is not part of the config file
    pub connection: ConnectionConfig,
    pub tables: BTreeMap<String, TableConfig<ClickHouseDataType>>,
    pub queries: BTreeMap<String, ParameterizedQueryConfig>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConnectionConfig {
    pub username: String,
    pub password: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableConfig<ColumnDataType> {
    /// The table name
    pub name: String,
    /// The table schema
    pub schema: String,
    /// Comments are sourced from the database table comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<PrimaryKey>,
    /// The map key is a column alias identifying the table and may be customized.
    /// It defaults to the table name.
    /// When the configuration is updated, the column is identified by name, and changes to the alias are preserved.
    pub columns: BTreeMap<String, ColumnConfig<ColumnDataType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PrimaryKey {
    pub name: String,
    /// The names of columns in this primary key
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnConfig<ColumnDataType> {
    /// The column name
    pub name: String,
    /// The column data type
    pub data_type: ColumnDataType,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
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
    pub return_type: ParameterizedQueryReturnType<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParameterizedQueryExposedAs {
    #[default]
    Collection,
    Procedure,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "definition", rename_all = "snake_case")]
pub enum ParameterizedQueryReturnType<DataType> {
    /// the same as the return type for a known table
    TableReference {
        /// the table alias must match a key in `tables`, and the query must return the same type as that table
        /// alternatively, the alias may reference another parameterized query which has a return type definition,
        table_alias: String,
    },
    /// The same as the return type for another query that has a return type definition
    QueryReference {
        /// the table alias must match a key in `tables`, and the query must return the same type as that table
        /// alternatively, the alias may reference another parameterized query which has a return type definition,
        query_alias: String,
    },
    /// A custom return type definition to associate with this query
    Custom { fields: BTreeMap<String, DataType> },
}

impl<T> Default for ParameterizedQueryReturnType<T> {
    fn default() -> Self {
        Self::Custom {
            fields: BTreeMap::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParameterizedQueryConfig {
    pub exposed_as: ParameterizedQueryExposedAs,
    pub comment: Option<String>,
    pub query: ParameterizedQuery,
    pub return_type: ParameterizedQueryReturnType<ClickHouseDataType>,
}

pub const CONFIG_FILE_NAME: &str = "configuration.json";
pub const CONFIG_SCHEMA_FILE_NAME: &str = "configuration.schema.json";
