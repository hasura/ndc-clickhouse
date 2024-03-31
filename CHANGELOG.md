# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Return error if empty list of query variables passed. Variables should be ommited or be a list with at least one member
- Use table comment as description for corresponding collection and object type
- Return json representation for applicable scalar types in schema response

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
