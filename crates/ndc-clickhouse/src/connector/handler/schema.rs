use crate::schema::{ClickHouseTypeDefinition, SchemaTypeDefinitions};
use common::{
    clickhouse_parser::{
        datatype::ClickHouseDataType,
        parameterized_query::{Parameter, ParameterType, ParameterizedQueryElement},
    },
    config::ServerConfig,
    config_file::{ParameterizedQueryExposedAs, PrimaryKey},
};
use ndc_sdk::{connector::SchemaError, json_response::JsonResponse, models};
use std::collections::BTreeMap;

pub async fn schema(
    configuration: &ServerConfig,
) -> Result<JsonResponse<models::SchemaResponse>, SchemaError> {
    let mut scalar_type_definitions = BTreeMap::new();
    let mut object_type_definitions = vec![];

    for (type_name, table_type) in &configuration.table_types {
        let mut fields = vec![];
        for (column_alias, column_type) in &table_type.columns {
            let type_definition = ClickHouseTypeDefinition::from_table_column(
                column_type,
                column_alias,
                type_name,
                &configuration.namespace_separator,
            );

            let SchemaTypeDefinitions { scalars, objects } = type_definition.type_definitions();

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
                column_alias.to_owned(),
                models::ObjectField {
                    description: None,
                    r#type: type_definition.type_identifier(),
                    arguments: BTreeMap::new(),
                },
            ));
        }

        object_type_definitions.push((
            type_name.to_owned(),
            models::ObjectType {
                description: table_type.comment.to_owned(),
                fields: fields.into_iter().collect(),
            },
        ));
    }

    for (table_alias, table_config) in &configuration.tables {
        for (argument_name, argument_type) in &table_config.arguments {
            let type_definition = ClickHouseTypeDefinition::from_query_argument(
                argument_type,
                argument_name,
                table_alias,
                &configuration.namespace_separator,
            );
            let SchemaTypeDefinitions { scalars, objects } = type_definition.type_definitions();

            for (name, definition) in objects {
                object_type_definitions.push((name, definition));
            }
            for (name, definition) in scalars {
                // silently dropping duplicate scalar definitions
                // this could be an issue if somehow an enum has the same name as a primitive scalar
                // there is the potential for name collisions resulting in dropped enum defintions
                scalar_type_definitions.insert(name, definition);
            }
        }
    }

    for (query_alias, query_config) in &configuration.queries {
        for element in &query_config.query.elements {
            if let ParameterizedQueryElement::Parameter(Parameter { name, r#type }) = element {
                let data_type = match r#type {
                    ParameterType::Identifier => &ClickHouseDataType::String,
                    ParameterType::DataType(t) => t,
                };
                let type_definition = ClickHouseTypeDefinition::from_query_argument(
                    data_type,
                    name.value(),
                    query_alias,
                    &configuration.namespace_separator,
                );

                let SchemaTypeDefinitions { scalars, objects } = type_definition.type_definitions();

                for (name, definition) in objects {
                    object_type_definitions.push((name, definition));
                }
                for (name, definition) in scalars {
                    // silently dropping duplicate scalar definitions
                    // this could be an issue if somehow an enum has the same name as a primitive scalar
                    // there is the potential for name collisions resulting in dropped enum defintions
                    scalar_type_definitions.insert(name, definition);
                }
            }
        }
    }

    let table_collections = configuration
        .tables
        .iter()
        .map(|(table_alias, table_config)| models::CollectionInfo {
            name: table_alias.to_owned(),
            description: table_config.comment.to_owned(),
            arguments: table_config
                .arguments
                .iter()
                .map(|(argument_name, argument_type)| {
                    let type_definition = ClickHouseTypeDefinition::from_query_argument(
                        argument_type,
                        argument_name,
                        table_alias,
                        &configuration.namespace_separator,
                    );
                    (
                        argument_name.to_owned(),
                        models::ArgumentInfo {
                            description: None,
                            argument_type: type_definition.type_identifier(),
                        },
                    )
                })
                .collect(),
            collection_type: table_config.return_type.to_owned(),
            uniqueness_constraints: table_config.primary_key.as_ref().map_or(
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
        });

    let query_collections = configuration
        .queries
        .iter()
        .filter(|(_, query_config)| {
            query_config.exposed_as == ParameterizedQueryExposedAs::Collection
        })
        .map(|(query_alias, query_config)| {
            // arguments with the same name may apear in multiple places in the same query
            // collecting into a map effectively de-duplicates the arguments
            let arguments = query_config
                .query
                .elements
                .iter()
                .filter_map(|element| match element {
                    ParameterizedQueryElement::String(_) => None,
                    ParameterizedQueryElement::Parameter(Parameter { name, r#type }) => {
                        let data_type = match r#type {
                            ParameterType::Identifier => &ClickHouseDataType::String,
                            ParameterType::DataType(t) => t,
                        };
                        let type_definition = ClickHouseTypeDefinition::from_query_argument(
                            data_type,
                            name.value(),
                            query_alias,
                            &configuration.namespace_separator,
                        );

                        Some((
                            name.value().to_owned(),
                            models::ArgumentInfo {
                                description: None,
                                argument_type: type_definition.type_identifier(),
                            },
                        ))
                    }
                })
                .collect();

            models::CollectionInfo {
                name: query_alias.to_owned(),
                description: query_config.comment.to_owned(),
                arguments,
                collection_type: query_config.return_type.to_owned(),
                uniqueness_constraints: BTreeMap::new(),
                foreign_keys: BTreeMap::new(),
            }
        });

    let collections = table_collections.chain(query_collections).collect();

    Ok(JsonResponse::Value(models::SchemaResponse {
        scalar_types: scalar_type_definitions,
        // converting vector to map drops any duplicate definitions
        // this could be an issue if there are name collisions
        object_types: object_type_definitions.into_iter().collect(),
        collections,
        functions: vec![],
        procedures: vec![],
    }))
}
