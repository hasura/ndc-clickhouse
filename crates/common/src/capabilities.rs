use ndc_models::{
    Capabilities, CapabilitiesResponse, ExistsCapabilities, LeafCapability, MutationCapabilities,
    NestedFieldCapabilities, QueryCapabilities, RelationshipCapabilities, VERSION,
};

pub fn capabilities() -> Capabilities {
    Capabilities {
        query: QueryCapabilities {
            aggregates: Some(LeafCapability {}),
            variables: Some(LeafCapability {}),
            explain: Some(LeafCapability {}),
            nested_fields: NestedFieldCapabilities {
                filter_by: None,
                order_by: None,
                aggregates: None,
            },
            exists: ExistsCapabilities {
                nested_collections: None,
            },
        },
        mutation: MutationCapabilities {
            transactional: None,
            explain: None,
        },
        relationships: Some(RelationshipCapabilities {
            relation_comparisons: Some(LeafCapability {}),
            order_by_aggregate: Some(LeafCapability {}),
        }),
    }
}

pub fn capabilities_response() -> CapabilitiesResponse {
    CapabilitiesResponse {
        version: VERSION.into(),
        capabilities: capabilities(),
    }
}
