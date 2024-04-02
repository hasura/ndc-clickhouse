use crate::schema::ClickHouseTypeDefinition;
use common::{
    clickhouse_parser::{
        datatype::{ClickHouseDataType, Identifier},
        parameterized_query::{Parameter, ParameterType, ParameterizedQueryElement},
    },
    config::{ParameterizedQueryExposedAs, ParameterizedQueryReturnType, PrimaryKey, ServerConfig},
};
use ndc_sdk::{connector::SchemaError, models};
use std::collections::BTreeMap;

pub async fn schema(configuration: &ServerConfig) -> Result<models::SchemaResponse, SchemaError> {
    let mut scalar_type_definitions = BTreeMap::new();
    let mut object_type_definitions = vec![];

    for (table_alias, table_config) in &configuration.tables {
        let mut fields = vec![];

        for (column_alias, column_config) in &table_config.columns {
            let type_definition = ClickHouseTypeDefinition::from_table_column(
                &column_config.data_type,
                &column_alias,
                &table_alias,
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
                column_alias.to_owned(),
                models::ObjectField {
                    description: None,
                    r#type: type_definition.type_identifier(),
                },
            ));
        }

        object_type_definitions.push((
            table_alias.to_owned(),
            models::ObjectType {
                description: table_config.comment.to_owned(),
                fields: fields.into_iter().collect(),
            },
        ));
    }

    for (query_alias, query_config) in &configuration.queries {
        if let ParameterizedQueryReturnType::Custom { fields } = &query_config.return_type {
            let mut query_type_fields = vec![];

            for (field_alias, field_data_type) in fields {
                let type_definition = ClickHouseTypeDefinition::from_table_column(
                    field_data_type,
                    field_alias,
                    query_alias,
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

                query_type_fields.push((
                    field_alias.to_owned(),
                    models::ObjectField {
                        description: None,
                        r#type: type_definition.type_identifier(),
                    },
                ));
            }
        }

        for element in &query_config.query.elements {
            if let ParameterizedQueryElement::Parameter(Parameter { name, r#type }) = element {
                let argument_alias = match name {
                    Identifier::DoubleQuoted(n)
                    | Identifier::BacktickQuoted(n)
                    | Identifier::Unquoted(n) => n,
                };
                let data_type = match r#type {
                    ParameterType::Identifier => &ClickHouseDataType::String,
                    ParameterType::DataType(t) => t,
                };
                let type_definition = ClickHouseTypeDefinition::from_query_argument(
                    data_type,
                    &argument_alias,
                    query_alias,
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
            }
        }
    }

    let table_collections = configuration
        .tables
        .iter()
        .map(|(table_alias, table_config)| models::CollectionInfo {
            name: table_alias.to_owned(),
            description: table_config.comment.to_owned(),
            arguments: BTreeMap::new(),
            collection_type: table_alias.to_owned(),
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
            let arguments = query_config
                .query
                .elements
                .iter()
                .filter_map(|element| match element {
                    ParameterizedQueryElement::String(_) => None,
                    ParameterizedQueryElement::Parameter(Parameter { name, r#type }) => {
                        let argument_alias = match name {
                            Identifier::DoubleQuoted(n)
                            | Identifier::BacktickQuoted(n)
                            | Identifier::Unquoted(n) => n,
                        };
                        let data_type = match r#type {
                            ParameterType::Identifier => &ClickHouseDataType::String,
                            ParameterType::DataType(t) => &t,
                        };
                        let type_definition = ClickHouseTypeDefinition::from_query_argument(
                            data_type,
                            &argument_alias,
                            query_alias,
                        );

                        Some((
                            argument_alias.to_owned(),
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
                collection_type: match &query_config.return_type {
                    ParameterizedQueryReturnType::TableReference {
                        table_alias: target_alias,
                    }
                    | ParameterizedQueryReturnType::QueryReference {
                        query_alias: target_alias,
                    } => target_alias.to_owned(),
                    ParameterizedQueryReturnType::Custom { .. } => query_alias.to_owned(),
                },
                uniqueness_constraints: BTreeMap::new(),
                foreign_keys: BTreeMap::new(),
            }
        });

    let collections = table_collections.chain(query_collections).collect();

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
