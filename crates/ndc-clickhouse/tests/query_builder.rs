use common::config_file::ServerConfigFile;
use ndc_clickhouse::sql::QueryBuilderError;
use ndc_sdk::models;
use schemars::schema_for;
use std::error::Error;
use test_utils::{test_error, test_generated_sql};
use tokio::fs;

mod test_utils {
    use common::config::ServerConfig;
    use ndc_clickhouse::{
        connector::read_server_config,
        sql::{QueryBuilder, QueryBuilderError},
    };
    use ndc_sdk::models::{self, QueryRequest};
    use std::{env, error::Error, path::PathBuf};
    use tokio::fs;

    /// when running tests locally, this can be set to true to update reference files
    /// this allows us to view diffs between commited samples and fresh samples
    /// we don't want that behavior when running CI, so this value should be false in commited code
    const UPDATE_GENERATED_SQL: bool = false;

    fn base_path(group_dir: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("query_builder")
            .join(group_dir)
    }
    async fn read_mock_configuration(group_dir: &str) -> Result<ServerConfig, Box<dyn Error>> {
        // set mock values for required env vars, we won't be reading these anyways
        env::set_var("CLICKHOUSE_URL", "");
        env::set_var("CLICKHOUSE_USERNAME", "");
        env::set_var("CLICKHOUSE_PASSWORD", "");
        let config_dir = base_path(group_dir).join("config");
        let configuration = read_server_config(config_dir).await?;
        Ok(configuration)
    }
    async fn read_request(
        group_dir: &str,
        test_name: &str,
    ) -> Result<QueryRequest, Box<dyn Error>> {
        let request_path = base_path(group_dir).join(format!("{test_name}.request.json"));

        let file_content = fs::read_to_string(request_path).await?;
        let request: models::QueryRequest = serde_json::from_str(&file_content)?;

        Ok(request)
    }
    async fn read_expected_sql(group_dir: &str, test_name: &str) -> Result<String, Box<dyn Error>> {
        let statement_path = base_path(group_dir).join(format!("{test_name}.statement.sql"));
        let expected_statement = fs::read_to_string(&statement_path).await?;
        Ok(expected_statement)
    }
    async fn write_expected_sql(
        group_dir: &str,
        test_name: &str,
        generated_statement: &str,
    ) -> Result<(), Box<dyn Error>> {
        let statement_path = base_path(group_dir).join(format!("{test_name}.statement.sql"));
        let pretty_statement = pretty_print_sql(&generated_statement);
        fs::write(&statement_path, &pretty_statement).await?;
        Ok(())
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
    fn generate_sql(
        configuration: &ServerConfig,
        request: &QueryRequest,
    ) -> Result<String, QueryBuilderError> {
        let generated_statement = pretty_print_sql(
            &QueryBuilder::new(&request, &configuration)
                .build()?
                .to_unsafe_sql_string(),
        );
        Ok(generated_statement)
    }
    pub async fn test_generated_sql(
        group_dir: &str,
        test_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let configuration = read_mock_configuration(group_dir).await?;
        let request = read_request(group_dir, test_name).await?;

        let generated_sql = generate_sql(&configuration, &request)?;

        let expected_sql = read_expected_sql(group_dir, test_name).await?;

        if UPDATE_GENERATED_SQL {
            write_expected_sql(group_dir, test_name, &generated_sql).await?;
        }

        assert_eq!(generated_sql, expected_sql);

        Ok(())
    }
    pub async fn test_error(
        group_dir: &str,
        test_name: &str,
        err: QueryBuilderError,
    ) -> Result<(), Box<dyn Error>> {
        let configuration = read_mock_configuration(group_dir).await?;
        let request = read_request(group_dir, test_name).await?;

        let result = generate_sql(&configuration, &request);

        assert_eq!(result, Err(err));

        Ok(())
    }
}

#[tokio::test]
#[ignore]
async fn update_json_schema() -> Result<(), Box<dyn Error>> {
    fs::write(
        "./tests/query_builder/request.schema.json",
        serde_json::to_string_pretty(&schema_for!(models::QueryRequest))?,
    )
    .await?;
    fs::write(
        "./tests/query_builder/configuration.schema.json",
        serde_json::to_string_pretty(&schema_for!(ServerConfigFile))?,
    )
    .await?;

    Ok(())
}
#[tokio::test]
async fn generate_column_accessor() -> Result<(), Box<dyn Error>> {
    test_generated_sql("field_selector", "01_generate_column_accessor").await
}
#[tokio::test]
async fn skip_if_not_required() -> Result<(), Box<dyn Error>> {
    test_generated_sql("field_selector", "02_skip_if_not_required").await
}
#[tokio::test]
async fn support_relationships_on_nested_field() -> Result<(), Box<dyn Error>> {
    test_generated_sql("field_selector", "03_support_relationships_on_nested_field").await
}
/// We do not support relationships on nested fileds if an array has been traversed
#[tokio::test]
async fn error_on_relationships_on_array_nested_field() -> Result<(), Box<dyn Error>> {
    let err =
        QueryBuilderError::NotSupported("Relationships with fields nested in arrays".to_string());
    test_error(
        "field_selector",
        "04_error_on_relationships_on_array_nested_field",
        err,
    )
    .await
}
#[tokio::test]
async fn complex_example() -> Result<(), Box<dyn Error>> {
    test_generated_sql("field_selector", "05_complex_example").await
}
#[tokio::test]
async fn no_useless_nested_accessors() -> Result<(), Box<dyn Error>> {
    test_generated_sql("field_selector", "06_no_useless_nested_accessors").await
}
