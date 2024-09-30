use common::{capabilities::capabilities_response, config_file::ServerConfigFile};
use insta::assert_yaml_snapshot;
use schemars::schema_for;

#[test]
fn test_capabilities() {
    assert_yaml_snapshot!("Capabilities", capabilities_response())
}

#[test]
fn test_configuration_schema() {
    assert_yaml_snapshot!("Server Configuration File", schema_for!(ServerConfigFile))
}
