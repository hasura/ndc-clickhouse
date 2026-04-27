# https://github.com/LukeMathWalker/cargo-chef
FROM rust:1.88.0 as chef
RUN cargo install --locked cargo-chef
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

FROM ubuntu:24.04 AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/ndc-clickhouse /usr/local/bin

RUN mkdir -p /etc/connector
ENV HASURA_CONFIGURATION_DIRECTORY=/etc/connector

ENTRYPOINT [ "/usr/local/bin/ndc-clickhouse" ]
CMD [ "serve" ]
