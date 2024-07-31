use ndc_sdk::models::{self, LeafCapability, NestedFieldCapabilities, RelationshipCapabilities};

pub fn capabilities() -> models::CapabilitiesResponse {
    models::CapabilitiesResponse {
        version: "0.1.4".to_owned(),
        capabilities: models::Capabilities {
            query: models::QueryCapabilities {
                aggregates: Some(LeafCapability {}),
                variables: Some(LeafCapability {}),
                explain: Some(LeafCapability {}),
                nested_fields: NestedFieldCapabilities {
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
