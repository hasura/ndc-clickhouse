use ndc_sdk::models::{
    Capabilities, CapabilitiesResponse, LeafCapability, MutationCapabilities,
    NestedFieldCapabilities, QueryCapabilities, RelationshipCapabilities,
};

pub fn capabilities() -> CapabilitiesResponse {
    CapabilitiesResponse {
        version: "0.1.4".to_owned(),
        capabilities: Capabilities {
            query: QueryCapabilities {
                aggregates: Some(LeafCapability {}),
                variables: Some(LeafCapability {}),
                explain: Some(LeafCapability {}),
                nested_fields: NestedFieldCapabilities {
                    filter_by: None,
                    order_by: None,
                    aggregates: None,
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
        },
    }
}
