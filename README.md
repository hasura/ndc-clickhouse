# Clickhouse Connector

The [Clickhouse](https://clickhouse.com/) Native Data Connector allows for connecting to a Clickhouse instance giving 
you an instant GraphQL API on top of your Clickhouse data.

This uses the [Rust Data Connector SDK](https://github.com/hasura/ndc-hub#rusk-sdk) from the [Data connector Hub](https://github.com/hasura/ndc-hub) and implements the [Data Connector Spec](https://github.com/hasura/ndc-spec).

- [Clickhouse Connector information in the Hasura Connectors directory](https://hasura.io/connectors/clickhouse)
- [Hasura V3 Documentation](https://hasura.io/docs/3.0)

In order to use this connector you will need to:

- Create a [Clickhouse account](https://clickhouse.cloud/signUp?loc=nav-get-started)
- Log in to a [Hasura CLI](https://hasura.io/docs/3.0/cli/overview/) Session
- Create a Pre-Shared Token for service authentication between the Hasura V3 Engine and your connector

## Features

TODO

## For Hasura Users

If you wish to use this connector with your Hasura projects, the best instructions can be found on the Hasura Hub 
(TODO: Link to hub page for Clickhouse Connector).

The following steps will allow you to deploy the connector and use it in a Hasura V3 project:

- Create a Hasura V3 Project (or use an existing project)
- Ensure that you have a metadata definition
- Create a configuration for the Clickhouse Connector referencing your credentials:
  `clickhouse.connector.configuration.json`
  You have 2 options for the config:
  1.  The easiest option is to is to run the connector locally in config mode:
  ```
  cargo run -- configuration serve --port 5000
  curl -X POST -H 'Content-Type: application/json' -d '{"connection": {"username": "your_username", "password": "your_password", "url": "your_clickhouse_url"}, "tables": []}' http://localhost:5000 > clickhouse.connector.configuration.json
  ```
  2.  The other option is to manually write your config that follows this pattern:
  ```
  {
     "connection": {
       "username": "your_username",
       "password": "your_password",
       "url": "your_clickhouse_url"
     },
     "tables": [
       {
         "name": "TableName",
         "schema": "SchemaName",
         "alias": "TableAlias",
         "primary_key": { "name": "TableId", "columns": ["TableId"] },
         "columns": [
           { "name": "TableId", "alias": "TableId", "data_type": "Int32" },
         ]
       }
     ]
   }
  ```
- Run the following command to deploy the connector
- Ensure you are logged in to Hasura CLI
  ```
  hasura3 cloud login --pat 'YOUR-HASURA-TOKEN'
  ```
- Deploy the connector
  ```
  hasura3 connector create clickhouse:v1 \
  --github-repo-url https://github.com/hasura/ndc-clickhouse/tree/main \
  --config-file ./clickhouse.connector.configuration.json
  ```
- Ensure that your deployed connector is referenced from your metadata with the service token. This can be done by adding a new file under `subgraphs/<YOUR_SUBGRAPH_DIR>/dataconnectors` with the following: 

```
kind: DataConnector
version: v1
definition:
  name: clickhouse
  url:
    singleUrl: <DEPLOYED_CONNECTOR_URL>
```
- Edit your metadata using the [LSP](https://marketplace.visualstudio.com/items?itemName=HasuraHQ.hasura) support to import the defined schema by running the comamnd `Hasura: Refresh Data Connector`. You can also track functions and procedures using the LSP command `Hasura: Track All`. 
- Deploy or update your Hasura cloud project
  ```
  hasura3 cloud build create --project-id my-project-id \
  --metadata-file metadata.json my-build-id
  ```
- View in your cloud console, access via the graphql API

## For Developers

The following instructions are for developers who wish to contribute to the Clickhouse Connector.

### Build

Prerequisites:

1. Install [rustup](https://www.rust-lang.org/tools/install).

Commands:

```
cargo build
cargo run -- configuration serve --port 5000
curl -X POST -d '{"connection": {"username": "your_username", "password": "your_password", "url": "your_clickhouse_url"}, "tables": []}' http://localhost:5000 > clickhouse.connector.configuration.json
cargo run -- serve --configuration clickhouse.connector.congifuration.json
```

### Docker

The `Dockerfile` is used by the `connector create` command and can be tested as follows:

```
docker build . --tag ndc-clickhouse
docker run -it --v ./clickhouse.connector.configuration.json:/config.json ndc-clickhouse
```

## Documentation

View other documentation for the ClickHouse connector [here](./docs/index.md).

## Contributing

Check out our [contributing guide](./docs/contributing.md) for more details.

## Support

Checkout out the [support section in docs](./docs/support.md).

## License

The ClickHouse connector is available under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
