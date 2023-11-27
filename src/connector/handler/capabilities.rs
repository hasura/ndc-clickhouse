use ndc_sdk::models::{self, LeafCapability, RelationshipCapabilities};

pub fn capabilities() -> models::CapabilitiesResponse {
    let package_version: &'static str = env!("CARGO_PKG_VERSION");
    models::CapabilitiesResponse {
        versions: package_version.to_string(),
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
