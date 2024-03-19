use std::{env, error::Error, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use ndc_clickhouse::connector::config::{update_tables_config, ConnectionConfig};
use tokio::fs;
mod metadata;

/// The release version specified at build time.
///
/// We should use the latest version if this is not specified.
const RELEASE_VERSION: Option<&str> = option_env!("RELEASE_VERSION");

#[derive(Parser)]
struct CliArgs {
    /// The PAT token which can be used to make authenticated calls to Hasura Cloud
    #[arg(long = "ddn-pat", value_name = "PAT", env = "HASURA_PLUGIN_DDN_PAT")]
    ddn_pat: Option<String>,
    /// If the plugins are sending any sort of telemetry back to Hasura, it should be disabled if this is true.
    #[arg(long = "disable-telemetry", env = "HASURA_PLUGIN_DISABLE_TELEMETRY")]
    disable_telemetry: bool,
    /// A UUID for every unique user. Can be used in telemetry
    #[arg(
        long = "instance-id",
        value_name = "ID",
        env = "HASURA_PLUGIN_INSTANCE_ID"
    )]
    instance_id: Option<String>,
    /// A UUID unique to every invocation of Hasura CLI
    #[arg(
        long = "execution-id",
        value_name = "ID",
        env = "HASURA_PLUGIN_EXECUTION_ID"
    )]
    execution_id: Option<String>,
    #[arg(
        long = "log-level",
        value_name = "LEVEL",
        env = "HASURA_PLUGIN_LOG_LEVEL",
        default_value = "info"
    )]
    log_level: LogLevel,
    /// Fully qualified path to the context directory of the connector
    #[arg(
        long = "connector-context-path",
        value_name = "PATH",
        env = "HASURA_PLUGIN_CONNECTOR_CONTEXT_PATH"
    )]
    context_path: Option<PathBuf>,
    #[arg(long = "clickhouse-url", value_name = "URL", env = "CLICKHOUSE_URL")]
    clickhouse_url: String,
    #[arg(long = "clickhouse-username", value_name = "USERNAME", env = "CLICKHOUSE_USERNAME", default_value_t = String::from("default"))]
    clickhouse_username: String,
    #[arg(
        long = "clickhouse-password",
        value_name = "PASSWORD",
        env = "CLICKHOUSE_PASSWORD"
    )]
    clickhouse_password: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        /// Whether to create the hasura connector metadata.
        with_metadata: bool,
    },
    Update {},
    Validate {},
    Watch {},
}

#[derive(Clone, ValueEnum)]
enum LogLevel {
    Panic,
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

struct Context {
    context_path: PathBuf,
    connection: ConnectionConfig,
    release_version: Option<&'static str>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();

    let context_path = match args.context_path {
        None => env::current_dir()?,
        Some(path) => path,
    };

    let connection = ConnectionConfig {
        url: args.clickhouse_url,
        username: args.clickhouse_username,
        password: args.clickhouse_password,
    };

    let context = Context {
        context_path,
        connection,
        release_version: RELEASE_VERSION,
    };

    match args.command {
        Command::Init { with_metadata } => {
            update_tables_config(&context.context_path, &context.connection).await?;
            // if requested, create the metadata
            if with_metadata {
                create_metadata(&context).await?;
            }
        }
        Command::Update {} => {
            update_tables_config(&context.context_path, &context.connection).await?;
        }
        Command::Validate {} => {
            todo!("implement validate command")
        }
        Command::Watch {} => {
            todo!("implement watch command")
        }
    }

    Ok(())
}

async fn create_metadata(context: &Context) -> Result<(), Box<dyn Error>> {
    let metadata_dir = context.context_path.join(".hasura-connector");
    match fs::create_dir(&metadata_dir).await {
        Err(err) => match err.kind() {
            // swallow error if directory already exists
            std::io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(err),
        },
        Ok(()) => Ok(()),
    }?;
    let metadata_file = metadata_dir.join("connector-metadata.yaml");
    let metadata = metadata::ConnectorMetadataDefinition {
        packaging_definition: metadata::PackagingDefinition::PrebuiltDockerImage(
            metadata::PrebuiltDockerImagePackaging {
                docker_image: format!(
                    "ghcr.io/hasura/ndc-clickhouse:{}",
                    context.release_version.unwrap_or("latest")
                ),
            },
        ),
        supported_environment_variables: vec![
            metadata::EnvironmentVariableDefinition {
                name: "CLICKHOUSE_URL".to_string(),
                description: "The ClickHouse connection url".to_string(),
                default_value: None,
            },
            metadata::EnvironmentVariableDefinition {
                name: "CLICKHOUSE_USERNAME".to_string(),
                description: "The ClickHouse connection username".to_string(),
                default_value: Some("default".to_string()),
            },
            metadata::EnvironmentVariableDefinition {
                name: "CLICKHOUSE_PASSWORD".to_string(),
                description: "The ClickHouse connection password".to_string(),
                default_value: None,
            },
        ],
        commands: metadata::Commands {
            update: Some("hasura-ndc-clickhouse update".to_string()),
            watch: None,
        },
        cli_plugin: Some(metadata::CliPluginDefinition {
            name: "ndc-clickhouse".to_string(),
            version: context.release_version.unwrap_or("latest").to_string(),
        }),
        docker_compose_watch: vec![metadata::DockerComposeWatchItem {
            path: "./".to_string(),
            target: Some("/etc/connector".to_string()),
            action: metadata::DockerComposeWatchAction::SyncAndRestart,
            ignore: vec![],
        }],
    };

    fs::write(metadata_file, serde_yaml::to_string(&metadata)?).await?;

    Ok(())
}
