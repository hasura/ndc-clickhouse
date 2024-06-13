# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.9]

- Change namespaceing to use `.` separator instead of `_`. We assume table names are less likely to contain periods, so this reduces likelyhood of naming conflicts.This will change generated type names and will thus manifest as a breaking change for some users
- Support `Nested` column types correctly, (previously these were treated as essentially Tuple columns)
- Support subfield selection on complex column types.
- Add support for relationships on column subfields, if the path to the subfield does not include lists
- Fix datatype parser: add ability to parse SimpleAggregateFunction and AggregateFunction columns

## [0.2.8]

- Make spans visible to cloud console users (tag spans with `internal.visibility = 'user'`)

## [0.2.7]

- Fix a bug introduced in 0.2.6 that would cause responses including a single quote to be serialized as invalid JSON strings

## [0.2.6]

- Add additional trace spans for SQL generation and query execution
- Do not parse db response as JSON, instead send it back directly

## [0.2.5]

- Implement validate cli command
- Do not write out config file unless it has changed. This avoids issues with the ddn cli which watches for filechange events
- Cast variables to UUID before comparison to UUID columns. This enables remote relationships to UUID columns

## [0.2.4]

- Allow explain of invalid foreach queries. Will generate invalid SQL, for proper execution these should be parameterized

## [0.2.3]

- Fix ordering of result sets for foreach queries (remote joins)

## [0.2.2]

- Return error if empty list of query variables passed. Variables should be ommited or be a list with at least one member
- Use table comment as description for corresponding collection and object type
- Return json representation for applicable scalar types in schema response
- Add `configuration.schema.json` to generated configuration directory
- Bump ndc-spec dependency to 0.1.1
- Config breaking change: use maps for tables and columns list, rather than arrays. This should help avoid duplicate alias issues
- Move parsing column data types into configuration time and startup time, instead of query execution time. This should give error feedback earlier
- Allow tables and native query return types to be marked as identical to return types for other tables/queries
- Support parameterized views (don't support column valued arguments)
- Support parameterized native queries, except in foreach queries. Also don't support column valued arguments
- Change explain output so the console knows how to extract generated SQL and sql explain plan to display to the user
- Pretty print explain SQL output
- Fix a bug where no result sets where returned when foreach predicate didn't match any rows. Correct behavior: empty result set is returned

## [0.2.1]

### CLI

- ignore casing for log-level flag

### Server

- default to `serve` command
- default `HASURA_CONFIGURATION_DIRECTORY` to `/etc/connector`

### CI

- correct `connnector-definition.tgz` archive: make root of archive relative (was absolute)

## [0.2.0]

- DDN Beta release
- add cli plugin
- remove configuration server mode

## [0.1.1]

- DDN Alpha Release
