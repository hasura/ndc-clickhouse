use crate::schema::ClickHouseTypeDefinition;
use common::{
    clickhouse_datatype::ClickHouseDataType,
    config::{PrimaryKey, ServerConfig},
};
use ndc_sdk::{connector::SchemaError, models};
use std::{collections::BTreeMap, str::FromStr};

pub async fn schema(configuration: &ServerConfig) -> Result<models::SchemaResponse, SchemaError> {
    let mut scalar_type_definitions = BTreeMap::new();
    let mut object_type_definitions = vec![];

    for table in &configuration.tables {
        let mut fields = vec![];

        for column in &table.columns {
            let data_type = ClickHouseDataType::from_str(column.data_type.as_str())
                .map_err(|err| SchemaError::Other(Box::new(err)))?;
            let type_definition = ClickHouseTypeDefinition::from_table_column(
                &data_type,
                &column.alias,
                &table.alias,
            );

            let (scalars, objects) = type_definition.type_definitions();

            for (name, definition) in objects {
                object_type_definitions.push((name, definition));
            }
            for (name, definition) in scalars {
                // silently dropping duplicate scalar definitions
                // this could be an issue if somehow an enum has the same name as a primitive scalar
                // there is the potential for name collisions resulting in dropped enum defintions
                scalar_type_definitions.insert(name, definition);
            }

            fields.push((
                column.alias.to_owned(),
                models::ObjectField {
                    description: None,
                    r#type: type_definition.type_identifier(),
                },
            ));
        }

        object_type_definitions.push((
            table.alias.to_owned(),
            models::ObjectType {
                description: table.comment.to_owned(),
                fields: fields.into_iter().collect(),
            },
        ));
    }

    let collections = configuration
        .tables
        .iter()
        .map(|table| models::CollectionInfo {
            name: table.alias.to_owned(),
            description: table.comment.to_owned(),
            arguments: BTreeMap::new(),
            collection_type: table.alias.to_owned(),
            uniqueness_constraints: table.primary_key.as_ref().map_or(
                BTreeMap::new(),
                |PrimaryKey { name, columns }| {
                    BTreeMap::from([(
                        name.to_owned(),
                        models::UniquenessConstraint {
                            unique_columns: columns.to_owned(),
                        },
                    )])
                },
            ),
            foreign_keys: BTreeMap::new(),
        })
        .collect();

    Ok(models::SchemaResponse {
        scalar_types: scalar_type_definitions,
        // converting vector to map drops any duplicate definitions
        // this could be an issue if there are name collisions
        object_types: object_type_definitions.into_iter().collect(),
        collections,
        functions: vec![],
        procedures: vec![],
    })
}
