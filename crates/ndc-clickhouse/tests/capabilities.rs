use common::capabilities::capabilities_response;
use ndc_sdk::models::CapabilitiesResponse;
use std::path::PathBuf;
use tokio::fs;

#[tokio::test]
async fn prints_expected_capabilities() {
    let expected_capabilities_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("capabilities")
        .join("capabilities.json");
    let expected_capabilities_file = fs::read_to_string(expected_capabilities_path)
        .await
        .expect("Capabilities file should be present");
    let expected_capabilities: CapabilitiesResponse =
        serde_json::from_str(&expected_capabilities_file)
            .expect("Expected capabilities shoudl be valid capabilities response");

    let capabilities = capabilities_response();

    assert_eq!(
        expected_capabilities, capabilities,
        "Capabilities response should match snapshot"
    );
}

#[tokio::test]
// update the expected capabilities. We only want thise called explicitly when capabilities have changed
#[ignore]
async fn update_expected_capabilities() {
    let capabilities = serde_json::to_string_pretty(&capabilities_response())
        .expect("Capabilities should serialize to json");

    let capabilities_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("capabilities")
        .join("capabilities.json");

    fs::write(capabilities_path, capabilities)
        .await
        .expect("Should write out capabilities");
}
