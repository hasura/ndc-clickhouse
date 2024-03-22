# Clickhouse Connector

**Compatible with Hasura DDN Alpha**

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

This native data connector implements the following Hasura Data Domain Specification features:

| Feature                                                                                                                             |     |
| ----------------------------------------------------------------------------------------------------------------------------------- | --- |
| [Simple Queries](https://hasura.io/docs/3.0/graphql-api/queries/simple-queries/)                                                    | ✅  |
| [Nested Queries](https://hasura.io/docs/3.0/graphql-api/queries/nested-queries/)                                                    | ✅  |
| [Query Result Sorting](https://hasura.io/docs/3.0/graphql-api/queries/sorting/)                                                     | ✅  |
| [Query Result Pagination](https://hasura.io/docs/3.0/graphql-api/queries/pagination/)                                               | ✅  |
| [Multiple Query Arguments](https://hasura.io/docs/3.0/graphql-api/queries/multiple-arguments/)                                      | ✅  |
| [Multiple Queries in a Request](https://hasura.io/docs/3.0/graphql-api/queries/multiple-queries/)                                   | ✅  |
| [Variables, Aliases, Fragments, Directives](https://hasura.io/docs/3.0/graphql-api/queries/variables-aliases-fragments-directives/) | ✅  |
| [Query Filter: Value Comparison](https://hasura.io/docs/3.0/graphql-api/queries/filters/comparison-operators/)                      | ✅  |
| [Query Filter: Boolean Expressions](https://hasura.io/docs/3.0/graphql-api/queries/filters/boolean-operators/)                      | ✅  |
| [Query Filter: Text](https://hasura.io/docs/3.0/graphql-api/queries/filters/text-search-operators/)                                 | ✅  |

## For Hasura Users

This connector should be used via the hasura ddn cli

## For Developers

The following instructions are for developers who wish to contribute to the Clickhouse Connector.

### Prerequisites:

1. Install [rustup](https://www.rust-lang.org/tools/install).
2. Install [docker](https://docs.docker.com/get-docker/).

### Use the CLI to create/update a configuration directory

View CLI help:

```sh
cargo run --package ndc-clickhouse-cli -- --help
```

Create a configuration directory in the `./config` directory:

```sh
cargo run --package ndc-clickhouse-cli -- init --context-path ./config --clickhouse-url "url" --clickhouse-username "user" --clickhouse-password "pass"
```

Update an existing directory. Will create the directory and files if not present.

This is required whenever the database schema changes

```sh
cargo run --package ndc-clickhouse-cli -- update --context-path ./config --clickhouse-url "url" --clickhouse-username "user" --clickhouse-password "pass"
```

### Run the connector server in docker

Create a `.env` file in the project root, replacing the placeholders with the actual values:

```env
CLICKHOUSE_URL=<value>
CLICKHOUSE_USERNAME=<value>
CLICKHOUSE_PASSWORD=<value>
```

Run the connector container. Check `docker-compose.yaml` for configuration details:

```sh
docker compose up -d
```

The connector should now be running and accepting requests.

To re-build the connector:

```sh
docker compose up -d --build
```

## Documentation

View other documentation for the ClickHouse connector [here](./docs/index.md).

## Contributing

Check out our [contributing guide](./docs/contributing.md) for more details.

## Support

Checkout out the [support section in docs](./docs/support.md).

## License

The ClickHouse connector is available under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
