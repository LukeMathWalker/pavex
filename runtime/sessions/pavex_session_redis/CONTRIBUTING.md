# Contributing Guide

This library relies on Redis for integration tests.\
Before running the test suite, you must have the database up and running.

---

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)

---

## Running the Test Databases

We provide a `docker-compose.yml` that starts Redis with predictable settings.

Start the services:

```sh
docker compose up -d
```

This will launch Redis (container: `test-pavex-session-redis`).

You can check its status with:

```sh
docker compose ps
```

and stop it when done with:

```sh
docker compose down
```

## Running Tests

Once the database is running, you can run the Rust test suite:

```sh
cargo test
```

The test code is configured to connect to `redis://127.0.0.1:56379`.

## Tips

- If you need to reset the databases, simply run:
  ```sh
  docker compose down -v
  docker compose up -d
  ```
- The containers have health checks configured, so they may take a few seconds before being ready.
  If you see connection errors, wait a few seconds and try again.
