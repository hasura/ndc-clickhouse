use ndc_sdk::models::{self, LeafCapability, RelationshipCapabilities};

pub fn capabilities() -> models::CapabilitiesResponse {
    models::CapabilitiesResponse {
        versions: "^0.1.1".to_string(),
        capabilities: models::Capabilities {
            query: models::QueryCapabilities {
                aggregates: Some(LeafCapability {}),
                variables: Some(LeafCapability {}),
            },
            explain: Some(LeafCapability {}),
            relationships: Some(RelationshipCapabilities {
                relation_comparisons: Some(LeafCapability {}),
                order_by_aggregate: Some(LeafCapability {}),
            }),
        },
    }
}
