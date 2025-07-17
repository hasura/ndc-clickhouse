use ndc_models::{
    AggregateCapabilities, Capabilities, CapabilitiesResponse, ExistsCapabilities, LeafCapability,
    MutationCapabilities, NestedFieldCapabilities, QueryCapabilities, RelationshipCapabilities,
    VERSION,
};

pub fn capabilities() -> Capabilities {
    Capabilities {
        query: QueryCapabilities {
            aggregates: Some(AggregateCapabilities {
                filter_by: None,
                group_by: None,
            }),
            variables: Some(LeafCapability {}),
            explain: Some(LeafCapability {}),
            nested_fields: NestedFieldCapabilities {
                filter_by: None,
                order_by: None,
                aggregates: None,
                nested_collections: None,
            },
            exists: ExistsCapabilities {
                nested_collections: None,
                unrelated: None,
                named_scopes: None,
                nested_scalar_collections: None,
            },
        },
        mutation: MutationCapabilities {
            transactional: None,
            explain: None,
        },
        relationships: Some(RelationshipCapabilities {
            relation_comparisons: Some(LeafCapability {}),
            order_by_aggregate: Some(LeafCapability {}),
            nested: None,
        }),
        relational_query: None,
        relational_mutation: None,
    }
}

pub fn capabilities_response() -> CapabilitiesResponse {
    CapabilitiesResponse {
        version: VERSION.into(),
        capabilities: capabilities(),
    }
}
