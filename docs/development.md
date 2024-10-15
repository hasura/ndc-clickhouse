# Development

This document details steps to running this connector with hasura ddn for local testing during connector development

**This document in now oudated**

## Prerequisites

- [ClickHouse Database](https://clickhouse.com/)
- [Rust Toolchain](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/get-docker/)
- [NGrok](https://ngrok.com/)
- [Hasura DDN Cli](https://hasura.io/docs/3.0/cli/installation)

## Clone the repository

Clone this repository to the directory of your choice

```sh
git clone https://github.com/hasura/ndc-clickhouse.git
```
All subsequent commands will assume they are running from the directory the repository was cloned into

```
cd ndc-clickhouse
```
## Generating a configuration directory

Create a `config` directory in the project root

```sh
mkdir config
```

Initialize the directory with a configuration based on your database schema

Change the placeholder auth details to your actual database connection information

```sh
cargo run --package ndc-clickhouse-cli -- --connector-context-path ./config --clickhouse-url "URL" --clickhouse-username "USERNAME" --clickhouse-password "PASSWORD" update
```

You can run this command again if your database schema has changed and you'd like the config to reflect that

Any configuration customization should not be overwritten

See also: [editing the configuration](./configuration.md)

## Running the connector in docker

Create a `.env` file in the project root

```.env
CLICKHOUSE_URL=<URL>
CLICKHOUSE_USERNAME=<USERNAME>
CLICKHOUSE_PASSWORD=<PASSWORD>
```
Start the connector

```sh
docker compose up -d
```

The first build may take a while. Subsequent builds should be faster thanks to layer caching

To restart the connector (required for changes to configuration to take effect)

```sh
docker compose restart
```

To rebuild the connector (required for changes to connector source code to take effect)

```sh
docker compose up -d --build
```

## Exposing the running connector to the web

We use `ngrok` to expose our connector to the web. You could use an alternative.

```sh
ngrok http http://localhost:4000
```
Take note of the resulting address, we'll need it.

See also: [ngrok documenation](https://ngrok.com/docs)

## Adding the connector to a ddn project

If you don't yet have a ddn project, you'll need to [create one](https://hasura.io/docs/3.0/getting-started/create-a-project#step-3-create-a-new-project)

The following instructions assume a default, empty project.
We will be adding a new datasource `clickhouse`. Change the name as needed

1. create a directory for the data source `app/clickhouse`
2. create the data source definition file `app/clickhouse/clickhouse.hml` with content:
```yaml
kind: DataConnectorLink
version: v1
definition:
    name: clickhouse
    url:
        Fn::ManifestRef: clickhouse
```
3. create the data source connector directory `app/clickhouse/connector`
4. create the data source connector file `app/clickhouse/connector/clickhouse.build.hml` with content:
```yaml
kind: ConnectorManifest
version: v1
spec:
  supergraphManifests:
  - base
definition:
  name: clickhouse
  type: endpoints
  deployments:
    - endpoint:
        valueFromEnv: CLICKHOUSE_CONNECTOR_ENDPOINT
```
4. add the `CLICKHOUSE_CONNECTOR_ENDPOINT` env var to `base.env.yaml`
```yaml
supergraph: {}
subgraphs:
  app:
    CLICKHOUSE_CONNECTOR_ENDPOINT: <endpoint>
```
The endpoint should be the one [exposed by NGROK](#exposing-the-running-connector-to-the-web)

## Building the ddn project

### Using ddn dev

This command will

- watch for changes in the connector schema
- track and update models whenever the schema changes
- create ddn builds whenever the metadata changes

```sh
ddn dev
```

You should now be able to navigate to your api

### Using ddn build

You can also replicate `ddn dev` step by step

**Updating the connector schema**

```sh
ddn update data-connector-link clickhouse
```
note here `clickhouse` is our source name, change it if needed

**Tracking models**

To explicitly track a model

```sh
ddn add model --data-connector-link clickhouse --name <model name>
```
The model name should be one of the collections exposed by the connector.
Check the `app/clickhouse/clickhouse.hml` file for a list.

note here `clickhouse` is our source name, change it if needed

**Creating a build**

```sh
ddn build connector-manifest
```

See also: [ddn documentation](https://hasura.io/docs/3.0)