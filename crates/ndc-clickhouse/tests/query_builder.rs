use common::config_file::ServerConfigFile;
use ndc_clickhouse::sql::QueryBuilderError;
use ndc_sdk::models;
use schemars::schema_for;
use std::error::Error;
use tokio::fs;

mod test_utils {
    use common::config::ServerConfig;
    use ndc_clickhouse::{
        connector::{handler::schema_response, setup::ClickhouseConnectorSetup},
        sql::{QueryBuilder, QueryBuilderError},
    };
    use ndc_sdk::models::{self, SchemaResponse};
    use std::{collections::HashMap, env, error::Error, path::PathBuf};
    use tokio::fs;

    /// when running tests locally, this can be set to true to update reference files
    /// this allows us to view diffs between commited samples and fresh samples
    /// we don't want that behavior when running CI, so this value should be false in commited code
    const UPDATE_SNAPSHOTS: bool = false;

    fn base_path(schema_dir: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("query_builder")
            .join(schema_dir)
    }
    fn tests_dir_path(schema_dir: &str, group_dir: &str) -> PathBuf {
        base_path(schema_dir).join(group_dir)
    }
    fn config_dir_path(schema_dir: &str) -> PathBuf {
        base_path(schema_dir).join("_config")
    }
    fn expected_schema_path(schema_dir: &str) -> PathBuf {
        base_path(schema_dir).join("_schema").join("schema.json")
    }
    async fn read_mock_configuration(schema_dir: &str) -> Result<ServerConfig, Box<dyn Error>> {
        // set mock values for required env vars, we won't be reading these anyways
        let env = HashMap::from_iter(vec![
            ("CLICKHOUSE_URL".to_owned(), "".to_owned()),
            ("CLICKHOUSE_USERNAME".to_owned(), "".to_owned()),
            ("CLICKHOUSE_PASSWORD".to_owned(), "".to_owned()),
        ]);
        let setup = ClickhouseConnectorSetup::new_from_env(env);
        let config_dir = config_dir_path(schema_dir);
        let configuration = setup.read_server_config(config_dir).await?;
        Ok(configuration)
    }
    async fn read_request(
        schema_dir: &str,
        group_dir: &str,
        test_name: &str,
    ) -> Result<models::QueryRequest, Box<dyn Error>> {
        let request_path =
            tests_dir_path(schema_dir, group_dir).join(format!("{test_name}.request.json"));

        let file_content = fs::read_to_string(request_path).await?;
        let request: models::QueryRequest = serde_json::from_str(&file_content)?;

        Ok(request)
    }
    async fn read_expected_sql(
        schema_dir: &str,
        group_dir: &str,
        test_name: &str,
    ) -> Result<String, Box<dyn Error>> {
        let statement_path =
            tests_dir_path(schema_dir, group_dir).join(format!("{test_name}.statement.sql"));
        let expected_statement = fs::read_to_string(&statement_path).await?;
        Ok(expected_statement)
    }
    async fn write_expected_sql(
        schema_dir: &str,
        group_dir: &str,
        test_name: &str,
        generated_statement: &str,
    ) -> Result<(), Box<dyn Error>> {
        let statement_path =
            tests_dir_path(schema_dir, group_dir).join(format!("{test_name}.statement.sql"));
        let pretty_statement = pretty_print_sql(generated_statement);
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
        request: &models::QueryRequest,
    ) -> Result<String, QueryBuilderError> {
        let generated_statement = pretty_print_sql(
            &QueryBuilder::new(request, configuration)
                .build_inlined()?
                .to_string(),
        );
        Ok(generated_statement)
    }
    pub async fn test_generated_sql(
        schema_dir: &str,
        group_dir: &str,
        test_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let configuration = read_mock_configuration(schema_dir).await?;
        let request = read_request(schema_dir, group_dir, test_name).await?;

        let generated_sql = generate_sql(&configuration, &request)?;

        if UPDATE_SNAPSHOTS {
            write_expected_sql(schema_dir, group_dir, test_name, &generated_sql).await?;
        } else {
            let expected_sql = read_expected_sql(schema_dir, group_dir, test_name).await?;

            assert_eq!(generated_sql, expected_sql);
        }

        Ok(())
    }
    pub async fn test_error(
        schema_dir: &str,
        group_dir: &str,
        test_name: &str,
        err: QueryBuilderError,
    ) -> Result<(), Box<dyn Error>> {
        let configuration = read_mock_configuration(schema_dir).await?;
        let request = read_request(schema_dir, group_dir, test_name).await?;

        let result = generate_sql(&configuration, &request);

        assert_eq!(result, Err(err));

        Ok(())
    }
    pub async fn test_schema(schema_dir: &str) -> Result<(), Box<dyn Error>> {
        let configuration = read_mock_configuration(schema_dir).await?;
        let schema = schema_response(&configuration);
        let expected_schema_path = expected_schema_path(schema_dir);

        if UPDATE_SNAPSHOTS {
            fs::write(expected_schema_path, serde_json::to_string_pretty(&schema)?).await?;
        } else {
            let expected_schema_file = fs::read_to_string(expected_schema_path).await?;
            let expected_schema: SchemaResponse = serde_json::from_str(&expected_schema_file)?;

            assert_eq!(schema, expected_schema, "Schema should match snapshot")
        }

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

#[cfg(test)]
mod schemas {
    use super::*;

    #[tokio::test]
    async fn chinook_schema() -> Result<(), Box<dyn Error>> {
        test_utils::test_schema("chinook").await
    }
    #[tokio::test]
    async fn complex_columns_schema() -> Result<(), Box<dyn Error>> {
        test_utils::test_schema("complex_columns").await
    }
    #[tokio::test]
    async fn star_schema_schema() -> Result<(), Box<dyn Error>> {
        test_utils::test_schema("star_schema").await
    }
}

#[cfg(test)]
mod simple_queries {
    use super::*;

    async fn test_generated_sql(name: &str) -> Result<(), Box<dyn Error>> {
        super::test_utils::test_generated_sql("chinook", "01_simple_queries", name).await
    }

    #[tokio::test]
    async fn select_rows() -> Result<(), Box<dyn Error>> {
        test_generated_sql("01_select_rows").await
    }
    #[tokio::test]
    async fn with_predicate() -> Result<(), Box<dyn Error>> {
        test_generated_sql("02_with_predicate").await
    }
    #[tokio::test]
    async fn larger_predicate() -> Result<(), Box<dyn Error>> {
        test_generated_sql("03_larger_predicate").await
    }
    #[tokio::test]
    async fn limit() -> Result<(), Box<dyn Error>> {
        test_generated_sql("04_limit").await
    }
    #[tokio::test]
    async fn offset() -> Result<(), Box<dyn Error>> {
        test_generated_sql("05_offset").await
    }
    #[tokio::test]
    async fn limit_offset() -> Result<(), Box<dyn Error>> {
        test_generated_sql("06_limit_offset").await
    }

    #[tokio::test]
    async fn order_by() -> Result<(), Box<dyn Error>> {
        test_generated_sql("07_order_by").await
    }
    #[tokio::test]
    async fn predicate_limit_offset_order_by() -> Result<(), Box<dyn Error>> {
        test_generated_sql("08_predicate_limit_offset_order_by").await
    }
}

#[cfg(test)]
mod relationships {
    use super::*;

    async fn test_generated_sql(name: &str) -> Result<(), Box<dyn Error>> {
        super::test_utils::test_generated_sql("chinook", "02_relationships", name).await
    }

    #[tokio::test]
    async fn object_relationship() -> Result<(), Box<dyn Error>> {
        test_generated_sql("01_object_relationship").await
    }
    #[tokio::test]
    async fn array_relationship() -> Result<(), Box<dyn Error>> {
        test_generated_sql("02_array_relationship").await
    }
    #[tokio::test]
    async fn parent_predicate() -> Result<(), Box<dyn Error>> {
        test_generated_sql("03_parent_predicate").await
    }
    #[tokio::test]
    async fn child_predicate() -> Result<(), Box<dyn Error>> {
        test_generated_sql("04_child_predicate").await
    }
    #[tokio::test]
    async fn traverse_relationship_in_predicate() -> Result<(), Box<dyn Error>> {
        test_generated_sql("05_traverse_relationship_in_predicate").await
    }
    #[tokio::test]
    async fn traverse_relationship_in_order_by() -> Result<(), Box<dyn Error>> {
        test_generated_sql("06_traverse_relationship_in_order_by").await
    }
    #[tokio::test]
    async fn order_by_aggregate_across_relationships() -> Result<(), Box<dyn Error>> {
        test_generated_sql("07_order_by_aggregate_across_relationships").await
    }
}

#[cfg(test)]
mod variables {
    use super::*;

    async fn test_generated_sql(name: &str) -> Result<(), Box<dyn Error>> {
        super::test_utils::test_generated_sql("chinook", "03_variables", name).await
    }

    #[tokio::test]
    async fn simple_predicate() -> Result<(), Box<dyn Error>> {
        test_generated_sql("01_simple_predicate").await
    }
    #[tokio::test]
    async fn empty_variable_sets() -> Result<(), Box<dyn Error>> {
        test_generated_sql("02_empty_variable_sets").await
    }
    #[tokio::test]
    async fn singe_set() -> Result<(), Box<dyn Error>> {
        test_generated_sql("03_single_set").await
    }
}

#[cfg(test)]
mod native_queries {
    use super::*;

    async fn test_generated_sql(name: &str) -> Result<(), Box<dyn Error>> {
        super::test_utils::test_generated_sql("star_schema", "01_native_queries", name).await
    }

    #[tokio::test]
    async fn native_query() -> Result<(), Box<dyn Error>> {
        test_generated_sql("01_native_query").await
    }
}

#[cfg(test)]
mod field_selector {
    use super::*;

    async fn test_generated_sql(name: &str) -> Result<(), Box<dyn Error>> {
        super::test_utils::test_generated_sql("complex_columns", "field_selector", name).await
    }
    async fn test_error(name: &str, err: QueryBuilderError) -> Result<(), Box<dyn Error>> {
        super::test_utils::test_error("complex_columns", "field_selector", name, err).await
    }

    #[tokio::test]
    async fn generate_column_accessor() -> Result<(), Box<dyn Error>> {
        test_generated_sql("01_generate_column_accessor").await
    }
    #[tokio::test]
    async fn skip_if_not_required() -> Result<(), Box<dyn Error>> {
        test_generated_sql("02_skip_if_not_required").await
    }
    #[tokio::test]
    async fn support_relationships_on_nested_field() -> Result<(), Box<dyn Error>> {
        test_generated_sql("03_support_relationships_on_nested_field").await
    }
    /// We do not support relationships on nested fileds if an array has been traversed
    #[tokio::test]
    async fn error_on_relationships_on_array_nested_field() -> Result<(), Box<dyn Error>> {
        let err = QueryBuilderError::NotSupported(
            "Relationships with fields nested in arrays".to_string(),
        );
        test_error("04_error_on_relationships_on_array_nested_field", err).await
    }
    #[tokio::test]
    async fn complex_example() -> Result<(), Box<dyn Error>> {
        test_generated_sql("05_complex_example").await
    }
    #[tokio::test]
    async fn no_useless_nested_accessors() -> Result<(), Box<dyn Error>> {
        test_generated_sql("06_no_useless_nested_accessors").await
    }
}
