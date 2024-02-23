# RealWorld example API

> An implementation of the [RealWorld API spec](https://www.realworld.how/) in Rust using the Pavex framework

# Overview

This codebase was created to demonstrate a fully fledged backend application built with **Rust** and [**Pavex**](https://pavex.dev) including CRUD operations, authentication, routing, pagination, and more.

You can exercise the application using Realworld's Postman collection: [here](https://github.com/gothinkster/realworld/tree/master/api).

For more information on how this works with other frontends/backends, head over to the [RealWorld](https://www.realworld.how/) website.

# Getting started

## Setup

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [`cargo-px`](https://lukemathwalker.github.io/cargo-px/)
- [Pavex](https://pavex.dev)
- [Docker](https://docs.docker.com/install/)
- [Postgres](https://www.postgresql.org/download/)
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
cargo px run
```

## Configuration

All configurable parameters are listed in `server/src/configuration/schema.rs`.

Configuration values are loaded from two sources:

- Configuration files
- Environment variables

Environment variables take precedence over configuration files.

All configuration files are in the `server/configuration` folder.
The application can be run in two different profiles: `dev` and `prod`.  
The settings that you want to share across all profiles should be placed
in `server/configuration/base.yml`.
Profile-specific configuration files can be then used
to override or supply additional values on top of the default settings (
e.g. `server/configuration/dev.yml`).

You can specify the app profile that you want to use by setting the `APP_PROFILE` environment variable; e.g.:

```bash
APP_PROFILE=prod cargo px run
```

for running the application with the `prod` profile.

By default, the `dev` profile is used since `APP_PROFILE` is set to `dev` in the `.env` file at the root of the project.
The `.env` file should not be committed to version control: it is meant to be used for local development only,
so that each developer can specify their own environment variables for secret values (e.g. database credentials)
that shouldn't be stored in configuration files (given their sensitive nature).
Since this an example, the `.env` file is committed for reference.

### Auth configuration

The application uses JWT for authentication, therefore it requires a secret key pair to sign and verify tokens. You can
generate one for local development purposes by running:

```bash 
openssl genpkey -algorithm ed25519 -out private.pem
openssl pkey -in private.pem -pubout -out public.pem
```

You then need to copy the generated keys to your configuration file, `api_server/configuration/dev.yml` for 
local development:

```yaml
# [...]
app:
  auth:
    eddsa_private_key_pem: |
      -----BEGIN PRIVATE KEY-----
      # Paste the contents of private.pem here
      -----END PRIVATE KEY-----
    eddsa_public_key_pem: |
      -----BEGIN PUBLIC KEY-----
      # Paste the contents of public.pem here
      -----END PUBLIC KEY-----
```