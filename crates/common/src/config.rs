use std::collections::BTreeMap;

use ndc_models::{ArgumentName, CollectionName, FieldName, ObjectTypeName};

use crate::{
    clickhouse_parser::{datatype::ClickHouseDataType, parameterized_query::ParameterizedQuery},
    config_file::{ParameterizedQueryExposedAs, PrimaryKey},
};

#[derive(Debug, Clone)]
/// In memory, runtime configuration, built from the configuration file(s) and environment variables
pub struct ServerConfig {
    /// the connection part of the config is not part of the config file
    pub connection: ConnectionConfig,
    pub namespace_separator: String,
    pub table_types: BTreeMap<ObjectTypeName, TableType>,
    pub tables: BTreeMap<CollectionName, TableConfig>,
    pub queries: BTreeMap<CollectionName, ParameterizedQueryConfig>,
}

#[derive(Debug, Clone)]
pub struct TableType {
    pub comment: Option<String>,
    pub columns: BTreeMap<FieldName, ClickHouseDataType>,
}

#[derive(Debug, Default, Clone)]
pub struct ConnectionConfig {
    pub username: String,
    pub password: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct TableConfig {
    /// The table name
    pub name: String,
    /// The table schema
    pub schema: String,
    /// Comments are sourced from the database table comment
    pub comment: Option<String>,
    pub primary_key: Option<PrimaryKey>,
    pub arguments: BTreeMap<ArgumentName, ClickHouseDataType>,
    // this key coresponds to a return type definition in the config table types
    pub return_type: ObjectTypeName,
}

#[derive(Debug, Clone)]
pub struct ParameterizedQueryConfig {
    pub exposed_as: ParameterizedQueryExposedAs,
    pub comment: Option<String>,
    pub query: ParameterizedQuery,
    pub return_type: ObjectTypeName,
}
