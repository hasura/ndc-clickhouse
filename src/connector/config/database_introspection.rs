use std::error::Error;

use serde::{Deserialize, Serialize};

use crate::connector::client::{execute_query, get_http_client};

use super::ConnectionConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct TableInfo {
    pub table_name: String,
    pub table_schema: String,
    pub table_catalog: String,
    pub table_type: TableType,
    pub primary_key: Option<String>,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub column_name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_in_primary_key: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TableType {
    #[serde(rename = "BASE TABLE")]
    Table,
    #[serde(rename = "VIEW")]
    View,
}

pub async fn introspect_database(
    connection_config: &ConnectionConfig,
) -> Result<Vec<TableInfo>, Box<dyn Error>> {
    let introspection_sql = include_str!("./database_introspection.sql");
    let client = get_http_client(connection_config)?;
    execute_query::<TableInfo>(&client, connection_config, introspection_sql, &vec![]).await
}
