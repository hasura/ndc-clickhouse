# Clickhouse NDC

An early stage ClickHouse Native Data Connector for Hasura V3

This uses the ndc_sdk from the [ndc-hub](https://github.com/hasura/ndc-hub) and implements the [ndc-spec](https://github.com/hasura/ndc-spec).

We provide a Dockerfile to build this connector, but as we cannot currently use multistage builds, the end image is quite large because it includes the entire rust toolchain...

See the included docker-compose file for an example of running the connector with a configuration file
