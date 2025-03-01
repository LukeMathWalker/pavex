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
      --version 0.8.0
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

## Auth configuration

The application uses JWT for authentication, therefore it requires a secret key pair to sign and verify tokens. You can
generate one for local development purposes by running:

```bash
openssl genpkey -algorithm ed25519 -out private.pem
openssl pkey -in private.pem -pubout -out public.pem
```

You then need to copy the generated keys to your configuration file, `configuration/dev.yml` for
local development:

```yaml
# [...]
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
