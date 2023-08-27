use ndc_sdk::models;
use serde_json::json;

pub fn capabilities() -> models::CapabilitiesResponse {
    let package_version: &'static str = env!("CARGO_PKG_VERSION");
    let empy_obj = || json!({});
    models::CapabilitiesResponse {
        versions: package_version.to_string(),
        capabilities: models::Capabilities {
            query: Some(models::QueryCapabilities {
                relation_comparisons: Some(empy_obj()),
                order_by_aggregate: Some(empy_obj()),
                foreach: Some(empy_obj()),
            }),
            explain: Some(empy_obj()),
            mutations: None,
            relationships: Some(empy_obj()),
        },
    }
}
