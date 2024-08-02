use std::error::Error;

use serde::Deserialize;

use common::{
    client::{execute_json_query, get_http_client},
    config::ConnectionConfig,
};

#[derive(Debug, Deserialize)]
pub struct TableInfo {
    pub table_name: String,
    pub table_schema: String,
    #[allow(dead_code)]
    pub table_catalog: String,
    pub table_comment: Option<String>,
    #[allow(dead_code)]
    pub table_type: TableType,
    pub primary_key: Option<String>,
    pub view_definition: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Deserialize)]
pub struct ColumnInfo {
    pub column_name: String,
    pub data_type: String,
    #[allow(dead_code)]
    pub is_nullable: bool,
    pub is_in_primary_key: bool,
}

#[derive(Debug, Deserialize)]
pub enum TableType {
    #[serde(
        rename = "BASE TABLE",
        alias = "FOREIGN TABLE",
        alias = "LOCAL TEMPORARY"
    )]
    Table,
    #[serde(rename = "VIEW", alias = "SYSTEM VIEW")]
    View,
}

pub async fn introspect_database(
    connection_config: &ConnectionConfig,
) -> Result<Vec<TableInfo>, Box<dyn Error>> {
    let introspection_sql = include_str!("./database_introspection.sql");
    let client = get_http_client(connection_config)?;
    execute_json_query::<TableInfo>(&client, connection_config, introspection_sql, &vec![]).await
}
