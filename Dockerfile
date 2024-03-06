# https://github.com/LukeMathWalker/cargo-chef
FROM rust:1.75.0 as chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src src 
RUN cargo build --locked --profile release --package ndc-clickhouse --bin server

FROM ubuntu:latest AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/server /usr/local/bin
ENTRYPOINT [ "/usr/local/bin/server" ]