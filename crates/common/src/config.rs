use std::{collections::BTreeMap, default};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::clickhouse_parser::{
    datatype::ClickHouseDataType, parameterized_query::ParameterizedQuery,
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
    pub tables: BTreeMap<String, TableConfig<String>>,
    /// Optionally define custom parameterized queries here
    /// Note the names must not match table names
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
    pub comment: Option<String>,
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
    kind: ParameterizedQueryKind,
    /// A relative path to a sql file
    file: String,
    /// Either a type definition for the return type for this query,
    /// or a reference to another return type: either a table's alias,
    /// or another query's alias. If another query, that query must have a return type definition.
    return_type: ParameterizedQueryReturnType<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, rename_all = "snake_case")]
pub enum ParameterizedQueryKind {
    #[default]
    Collection,
    Procedure,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParameterizedQueryReturnType<DataType> {
    /// the same as the return type for a known table
    Reference {
        /// the table alias must match a key in `tables`, and the query must return the same type as that table
        /// alternatively, the alias may reference another parameterized query which has a return type definition,
        alias: String,
    },
    Definition {
        fields: BTreeMap<String, DataType>,
    },
}

impl<T> Default for ParameterizedQueryReturnType<T> {
    fn default() -> Self {
        Self::Definition {
            fields: BTreeMap::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParameterizedQueryConfig {
    kind: ParameterizedQueryKind,
    query: ParameterizedQuery,
    return_type: ParameterizedQueryReturnType<ClickHouseDataType>,
}

pub const CONFIG_FILE_NAME: &str = "configuration.json";
pub const CONFIG_SCHEMA_FILE_NAME: &str = "configuration.schema.json";
