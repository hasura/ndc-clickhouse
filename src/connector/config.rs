use std::error::Error;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use self::database_introspection::{introspect_database, ColumnInfo, TableInfo};

mod database_introspection;

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfig {
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
    pub name: String,
    pub schema: String,
    pub alias: String,
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
    pub name: String,
    pub alias: String,
    pub data_type: String,
}

/// using a server config that may not contain any table information,
/// produce a new configuration that copies the connection information from the original
/// and additionally includes table configuration information obtained by introspecting the database
pub async fn get_server_config(
    connection_config: &ConnectionConfig,
) -> Result<ServerConfig, Box<dyn Error>> {
    let table_infos = introspect_database(connection_config).await?;

    let tables = table_infos
        .iter()
        .map(|table| TableConfig {
            name: table.table_name.to_owned(),
            schema: table.table_schema.to_owned(),
            alias: get_table_alias(table),
            primary_key: table.primary_key.as_ref().map(|primary_key| {
                PrimaryKey {
                    name: primary_key.to_owned(),
                    columns: table
                        .columns
                        .iter()
                        .filter_map(|column| {
                            if column.is_in_primary_key {
                                // note: we should alias the column here.
                                Some(get_column_alias(column, table))
                            } else {
                                None
                            }
                        })
                        .collect(),
                }
            }),
            columns: table
                .columns
                .iter()
                .map(|column| ColumnConfig {
                    name: column.column_name.to_owned(),
                    alias: get_column_alias(column, table),
                    data_type: column.data_type.to_owned(),
                })
                .collect(),
        })
        .collect();

    Ok(ServerConfig {
        tables,
        connection: connection_config.to_owned(),
    })
}

fn get_table_alias(table: &TableInfo) -> String {
    // TODO: ensure a valid graphql name, I think?
    // unsure if HGE will complain if the names are not valid graphql identifiers
    if table.table_schema == "default" {
        table.table_name.to_owned()
    } else {
        format!("{}_{}", table.table_schema, table.table_name)
    }
}
fn get_column_alias(column: &ColumnInfo, _table: &TableInfo) -> String {
    // TODO: ensure a valid graphql name, I think?
    // unsure if HGE will complain if the names are not valid graphql identifiers
    column.column_name.to_owned()
}

/// generate an updated configuration by introspecting the database
/// expects a generated_config.json file to exists with connection information.
/// If not such file exists, you can create one by using this template:
/// ```json
/// {
///   "connection": {
///     "username": "",
///     "password": "",
///     "url": ""
///   },
///   "tables": []
/// }
/// ```
#[tokio::test]
async fn update_config() -> Result<(), Box<dyn Error>> {
    let current_config = std::fs::read_to_string("./generated_config.json")
        .expect("Unable to read generated_config");

    let current_config: ServerConfig = serde_json::from_str(&current_config)?;

    let updated_config = get_server_config(&current_config.connection).await?;

    let updated_config = serde_json::to_string_pretty(&updated_config)?;

    std::fs::write("./generated_config.json", updated_config)
        .expect("Unable to write generated config");

    Ok(())
}
