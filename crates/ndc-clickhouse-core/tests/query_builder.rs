use common::{
    config::{read_server_config, ConfigurationEnvironment, ServerConfig},
    config_file::ServerConfigFile,
    schema::schema_response,
};
use insta::{assert_snapshot, assert_yaml_snapshot, glob};
use ndc_clickhouse_core::sql::QueryBuilder;
use ndc_models as models;
use schemars::schema_for;
use std::{collections::HashMap, error::Error, fs, path::PathBuf};

fn base_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("query_builder")
}

async fn read_mock_configuration(schema_dir: &str) -> ServerConfig {
    // set mock values for required env vars, we won't be reading these anyways
    let env = HashMap::from_iter(vec![
        ("CLICKHOUSE_URL".to_owned(), "".to_owned()),
        ("CLICKHOUSE_USERNAME".to_owned(), "".to_owned()),
        ("CLICKHOUSE_PASSWORD".to_owned(), "".to_owned()),
    ]);
    let config_dir = base_path().join(schema_dir).join("_config");
    read_server_config(
        config_dir.as_path(),
        &ConfigurationEnvironment::from_simulated_environment(env),
    )
    .await
    .expect("Should be able to read configuration")
}

fn pretty_print_sql(query: &str) -> String {
    use sqlformat::{format, FormatOptions, Indent, QueryParams};
    let params = QueryParams::None;
    let options = FormatOptions {
        indent: Indent::Spaces(2),
        uppercase: false,
        lines_between_queries: 1,
    };

    format(query, &params, options)
}

#[tokio::test]
async fn test_sql_generation() {
    for schema_dir in ["chinook", "complex_columns", "star_schema"] {
        let configuration = read_mock_configuration(schema_dir).await;

        glob!(
            base_path().join(schema_dir),
            "*.request.json",
            |file_path| {
                let file = fs::read_to_string(file_path).expect("Should read request file");
                let request: models::QueryRequest =
                    serde_json::from_str(&file).expect("File should be valid query request");

                let inlined_sql = match QueryBuilder::new(&request, &configuration).build_inlined()
                {
                    Err(err) => {
                        assert_snapshot!(format!("{schema_dir} Expected Error"), err);
                        return;
                    }
                    Ok(inlined_sql) => pretty_print_sql(&inlined_sql.to_string()),
                };

                assert_snapshot!(format!("{schema_dir} Inlined SQL"), inlined_sql);

                let (parameterized_sql, parameters) = QueryBuilder::new(&request, &configuration)
                    .build_parameterized()
                    .expect("Should build parameterized SQL");
                let parameterized_sql = pretty_print_sql(&parameterized_sql.to_string());

                if parameters.is_empty() {
                    assert_eq!(
                        inlined_sql, parameterized_sql,
                        "If no parameters are present, parameterized sql should match inlined sql"
                    )
                } else {
                    let printed_parameters =
                        parameters
                            .into_iter()
                            .fold(String::new(), |mut acc, (name, value)| {
                                acc.reserve(name.len() + value.len() + 2);
                                acc.push_str(&name);
                                acc.push('=');
                                acc.push_str(&value);
                                acc.push('\n');
                                acc
                            });

                    assert_snapshot!(format!("{schema_dir} Parameterized SQL"), parameterized_sql);
                    assert_snapshot!(format!("{schema_dir} Parameters"), printed_parameters);
                }
            }
        )
    }
}

#[tokio::test]
async fn test_schemas() {
    for schema_dir in ["chinook", "complex_columns", "star_schema"] {
        let configuration = read_mock_configuration(schema_dir).await;

        let schema = schema_response(&configuration);
        assert_yaml_snapshot!(format!("{schema_dir} Schema Response"), schema);
    }
}

#[tokio::test]
#[ignore]
async fn update_json_schema() -> Result<(), Box<dyn Error>> {
    fs::write(
        "./tests/query_builder/request.schema.json",
        serde_json::to_string_pretty(&schema_for!(models::QueryRequest))?,
    )?;
    fs::write(
        "./tests/query_builder/configuration.schema.json",
        serde_json::to_string_pretty(&schema_for!(ServerConfigFile))?,
    )?;

    Ok(())
}
