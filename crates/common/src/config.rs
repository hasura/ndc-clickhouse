use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::clickhouse_datatype::ClickHouseDataType;

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfigFile {
    #[serde(rename = "$schema")]
    pub schema: String,
    /// A list of tables available in this database
    pub tables: BTreeMap<String, TableConfig<String>>,
}

#[derive(Debug, Default, Clone)]
pub struct ServerConfig {
    /// the connection part of the config is not part of the config file
    pub connection: ConnectionConfig,
    /// The map key is a unique table alias that defaults to defaults to "<table_schema>_<table_name>",
    /// except for tables in the "default" schema where the table name is used
    /// This is the name exposed to the engine, and may be configured by users.
    /// When the configuration is updated, the table is identified by name and schema, and changes to the alias are preserved.
    pub tables: BTreeMap<String, TableConfig<ClickHouseDataType>>,
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

pub const CONFIG_FILE_NAME: &str = "configuration.json";
pub const CONFIG_SCHEMA_FILE_NAME: &str = "configuration.schema.json";
