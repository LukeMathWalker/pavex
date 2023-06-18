# RealWorld example API

> An implementation of the [RealWorld API spec](https://www.realworld.how/) in Rust using the Pavex framework

# Overview

This codebase was created to demonstrate a fully fledged backend application built with **Rust** and [**Pavex**](https://pavex.dev) including CRUD operations, authentication, routing, pagination, and more.

You can exercise the application using Realworld's Postman collection: [here](https://github.com/gothinkster/realworld/tree/master/api).

For more information on how this works with other frontends/backends, head over to the [RealWorld](https://www.realworld.how/) website.

# Getting started

## Setup

### Prerequisites

- Rust (see [here](https://www.rust-lang.org/tools/install) for instructions)
- Docker (see [here](https://docs.docker.com/install/) for instructions)
- Postgres (see [here](https://www.postgresql.org/download/) for instructions)
- `cargo-px`:
  ```bash
  cargo install cargo-px
  ```
- `pavex_cli`:
  ```bash
  cd ../../libs && cargo build --release -p pavex_cli  
  ```
- `sqlx` CLI:
  ```bash
  cargo install sqlx-cli \
      --no-default-features \
      --features native-tls,postgres \
      --version 0.7.0-alpha.3
  ```

### Setup steps
- Launch a local Postgres instance and run SQL migrations:
```bash
./scripts/init_db.sh
```

You are ready to go!

## Build the application

```bash
# Add --release if you're looking for maximum performance
cargo px build
```

## Run the application

```bash
APP_PROFILE=dev cargo px run --bin api
```

## Configuration

All configuration files are in the `api_server/configuration` folder.  
The default settings are stored in `api_server/configuration/base.yml`.

Environment-specific configuration files can be used to override or supply additional values on top the default settings (see `prod.yml`).  
You must specify the app profile that you want to use by setting the `APP_PROFILE` environment variable to either `dev`, `test` or `prod`; e.g.:

```bash
APP_PROFILE=prod cargo px run --bin api
```

All configurable parameters are listed in `api_server/src/configuration.rs`.
