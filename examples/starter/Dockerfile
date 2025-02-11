# `cargo-chef` is a cargo-subcommand that provides
# enhanced Docker layer caching for Rust projects.
FROM lukemathwalker/cargo-chef:latest AS chef
WORKDIR /app
# Force `rustup` to sync the toolchain in the base `chef` layer
# so that it doesn't happen more than once
COPY rust-toolchain.toml .
RUN rustup show active-toolchain

FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project's dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Build our project
RUN cargo build --release --package server --bin server

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/server bin
COPY server/configuration server/configuration
ENV APP_PROFILE=production
# Enable backtraces to simplify debugging
# production panics.
ENV RUST_BACKTRACE=1
# We don't want `anyhow` to capture backtraces for
# "routine" errors. Just panics.
ENV RUST_LIB_BACKTRACE=0
ENTRYPOINT ["./bin"]
