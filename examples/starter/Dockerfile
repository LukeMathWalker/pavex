# `cargo-chef` is a cargo-subcommand that provides
# enhanced Docker layer caching for Rust projects.
FROM lukemathwalker/cargo-chef:latest as chef
WORKDIR /app
# Force `rustup` to sync the toolchain in the base `chef` layer
# so that it doesn't happen more than once
COPY rust-toolchain.toml .
RUN cargo --version

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project's dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Build our project
RUN cargo build --release --bin server

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/server bin
COPY server/configuration server/configuration
ENV APP_PROFILE production
ENTRYPOINT ["./bin"]
