use std::collections::BTreeMap;

use ndc_sdk::models::{Argument, ArgumentName, CollectionName, Relationship, RelationshipArgument};

#[derive(Debug, Clone)]
pub enum CollectionContext<'a, 'b> {
    Base {
        collection_alias: &'a CollectionName,
        arguments: &'b BTreeMap<ArgumentName, Argument>,
    },
    Relationship {
        collection_alias: &'a CollectionName,
        arguments: &'b BTreeMap<ArgumentName, RelationshipArgument>,
        relationship_arguments: &'a BTreeMap<ArgumentName, RelationshipArgument>,
    },
    UnrelatedRelationship {
        collection_alias: &'a CollectionName,
        arguments: &'b BTreeMap<ArgumentName, RelationshipArgument>,
    },
}

impl<'a, 'b> CollectionContext<'a, 'b> {
    pub fn new(
        collection_alias: &'a CollectionName,
        arguments: &'b BTreeMap<ArgumentName, Argument>,
    ) -> Self {
        Self::Base {
            collection_alias,
            arguments,
        }
    }
    pub fn new_unrelated(
        collection_alias: &'a CollectionName,
        arguments: &'b BTreeMap<ArgumentName, RelationshipArgument>,
    ) -> Self {
        Self::UnrelatedRelationship {
            collection_alias,
            arguments,
        }
    }
    pub fn from_relationship(
        relationship: &'a Relationship,
        arguments: &'b BTreeMap<ArgumentName, RelationshipArgument>,
    ) -> Self {
        Self::Relationship {
            collection_alias: &relationship.target_collection,
            relationship_arguments: &relationship.arguments,
            arguments,
        }
    }
    pub fn alias(&self) -> &CollectionName {
        match self {
            CollectionContext::Base {
                collection_alias, ..
            }
            | CollectionContext::Relationship {
                collection_alias, ..
            }
            | CollectionContext::UnrelatedRelationship {
                collection_alias, ..
            } => collection_alias,
        }
    }
    pub fn has_arguments(&self) -> bool {
        match self {
            CollectionContext::Base {
                collection_alias: _,
                arguments,
            } => !arguments.is_empty(),
            CollectionContext::Relationship {
                collection_alias: _,
                arguments,
                relationship_arguments,
            } => !arguments.is_empty() || !relationship_arguments.is_empty(),
            CollectionContext::UnrelatedRelationship {
                collection_alias: _,
                arguments,
            } => !arguments.is_empty(),
        }
    }
}
