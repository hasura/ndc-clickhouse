#!/usr/bin/env bash

set -evo pipefail
ROOT="$(pwd)"

mkdir -p "${ROOT}/release/connector-definition/.hasura-connector/"
cat "${ROOT}/ci/templates/connector-metadata.yaml" | envsubst > "${ROOT}/release/connector-definition/.hasura-connector/connector-metadata.yaml"
tar -czvf "${ROOT}/release/artifacts/connector-definition.tgz" "${ROOT}/release/connector-definition/"