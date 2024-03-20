#!/usr/bin/env bash

set -evo pipefail
ROOT="$(pwd)"

export LINUX_AMD64_SHA256=$(sha256sum ${ROOT}/release/artifacts/ndc-clickhouse-cli-x86_64-unknown-linux-musl     | cut -f1 -d' ')
export MACOS_AMD64_SHA256=$(sha256sum ${ROOT}/release/artifacts/ndc-clickhouse-cli-x86_64-apple-darwin           | cut -f1 -d' ')
export WINDOWS_AMD64_SHA256=$(sha256sum ${ROOT}/release/artifacts/ndc-clickhouse-cli-x86_64-pc-windows-msvc.exe  | cut -f1 -d' ')
export LINUX_ARM64_SHA256=$(sha256sum ${ROOT}/release/artifacts/ndc-clickhouse-cli-aarch64-unknown-linux-musl    | cut -f1 -d' ')
export MACOS_ARM64_SHA256=$(sha256sum ${ROOT}/release/artifacts/ndc-clickhouse-cli-aarch64-apple-darwin          | cut -f1 -d' ')

mkdir -p "${ROOT}/release/"
cat "${ROOT}/ci/templates/manifest.yaml" | envsubst > "${ROOT}/release/manifest.yaml"
