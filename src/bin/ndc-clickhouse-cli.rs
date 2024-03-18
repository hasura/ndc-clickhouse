use std::error::Error;

use clap::{Parser, Subcommand, ValueEnum};
use ndc_clickhouse::connector::config::{update_tables_config, ConnectionConfig};

#[derive(Parser)]
struct CliArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Subcommand)]
enum Command {
    #[command()]
    Init(CliPluginArg),
    #[command()]
    Update(CliPluginArg),
    #[command()]
    Validate(CliPluginArg),
    #[command()]
    Watch(CliPluginArg),
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

#[derive(Clone, Parser)]
struct CliPluginArg {
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
    connector_context_path: String,
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let CliArgs { command } = CliArgs::parse();

    // https://github.com/hasura/ndc-hub/blob/cli-guidelines/rfcs/0002-cli-guidelines.md
    // todo: we can and should expect env vars for the clickhouse connection to already be set here.
    // we should validate those env vars, and use them to create the list of tables by introspecting the database
    match command {
        Command::Init(init_command) => todo!("create empty tables.json file"),
        // we probably want a mechanism allowing users to update/include only a subset of the database schema, but this can wait
        Command::Update(update_command) => {
            let configuration_dir = &update_command.connector_context_path;

            let connection = ConnectionConfig {
                url: update_command.clickhouse_url.to_owned(),
                username: update_command.clickhouse_username.to_owned(),
                password: update_command.clickhouse_password.to_owned(),
            };

            update_tables_config(configuration_dir, &connection).await?;
        }
        // unsure what we need here. Parse the config at least, maybe check it against the db?
        Command::Validate(validate_command) => todo!("validate tables.json"),
        // unsure how to use this. We don't want to run it in a real loop, maybe with a delay?
        // maybe this regularly checks if some file has been changed, and updates the introspection when needed
        Command::Watch(watch_command) => todo!(),
    };

    Ok(())
}
