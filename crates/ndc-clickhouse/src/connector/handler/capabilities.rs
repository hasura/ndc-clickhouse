use ndc_sdk::models::{self, LeafCapability, RelationshipCapabilities};

pub fn capabilities() -> models::CapabilitiesResponse {
    models::CapabilitiesResponse {
        version: "^0.1.1".to_string(),
        capabilities: models::Capabilities {
            query: models::QueryCapabilities {
                aggregates: Some(LeafCapability {}),
                variables: Some(LeafCapability {}),
                explain: Some(LeafCapability {}),
                nested_fields: models::NestedFieldCapabilities {
                    filter_by: None,
                    order_by: None,
                    aggregates: None,
                },
            },
            mutation: models::MutationCapabilities {
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
