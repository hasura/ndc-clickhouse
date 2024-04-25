# https://github.com/LukeMathWalker/cargo-chef
FROM rust:1.75.0 as chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG RUSTFLAGS
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --profile release --recipe-path recipe.json
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY crates crates
RUN cargo build --locked --profile release --package ndc-clickhouse

FROM ubuntu:latest AS runtime
RUN apt-get update && apt-get install -y ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/ndc-clickhouse /usr/local/bin

RUN mkdir -p /etc/connector
ENV HASURA_CONFIGURATION_DIRECTORY=/etc/connector

ENTRYPOINT [ "/usr/local/bin/ndc-clickhouse" ]
CMD [ "serve" ]