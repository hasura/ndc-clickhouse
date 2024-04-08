# Clickhouse Connector

**Compatible with Hasura DDN Beta**

The [Clickhouse](https://clickhouse.com/) Native Data Connector allows for connecting to a Clickhouse instance giving
you an instant GraphQL API on top of your Clickhouse data.

This uses the [Rust Data Connector SDK](https://github.com/hasura/ndc-hub#rusk-sdk) from the [Data connector Hub](https://github.com/hasura/ndc-hub) and implements the [Data Connector Spec](https://github.com/hasura/ndc-spec).

- [Clickhouse Connector information in the Hasura Connectors directory](https://hasura.io/connectors/clickhouse)
- [Hasura V3 Documentation](https://hasura.io/docs/3.0)

In order to use this connector you will need to:

- Create a [Clickhouse account](https://clickhouse.cloud/signUp?loc=nav-get-started)
- Log in to a [Hasura CLI](https://hasura.io/docs/3.0/cli/overview/) Session

## For Hasura Users

This connector should be used via the hasura ddn cli

See [configuration instructions](./docs/configuration.md) for additional configuration instructions

## For Developers

See [development instructions](./docs/development.md)

## Documentation

View other documentation for the ClickHouse connector [here](./docs/index.md).

## Contributing

Check out our [contributing guide](./docs/contributing.md) for more details.

## Support

Checkout out the [support section in docs](./docs/support.md).

## License

The ClickHouse connector is available under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
