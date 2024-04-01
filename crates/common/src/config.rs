use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfigFile {
    #[serde(rename = "$schema")]
    pub schema: String,
    /// A list of tables available in this database
    pub tables: Vec<TableConfig>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfig {
    /// the connection part of the config is not part of the config file
    pub connection: ConnectionConfig,
    pub tables: Vec<TableConfig>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConnectionConfig {
    pub username: String,
    pub password: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableConfig {
    /// The table name
    pub name: String,
    /// The table schema
    pub schema: String,
    /// The table alias defaults to "<table_schema>_<table_name>", except for tables in the "default" schema where the table name is used
    /// This is the name exposed to the engine, and may be configured by users. This is preserved through config updates
    pub alias: String,
    /// Comments are sourced from the database table comment
    pub comment: Option<String>,
    pub primary_key: Option<PrimaryKey>,
    pub columns: Vec<ColumnConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PrimaryKey {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnConfig {
    /// The column name
    pub name: String,
    /// The column alias defaults to the column name, but may be changed by users. This is preserved through config updates
    pub alias: String,
    /// The column data type
    pub data_type: String,
}

pub const CONFIG_FILE_NAME: &str = "configuration.json";
pub const CONFIG_SCHEMA_FILE_NAME: &str = "configuration.schema.json";
