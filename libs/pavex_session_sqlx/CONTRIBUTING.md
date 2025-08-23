# Contributing Guide

This library relies on PostgreSQL, MySQL and SQLite for integration tests.\
Before running the test suite, you must have the first two databases up and running.

---

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)

---

## Running the Test Databases

We provide a `docker-compose.yml` that starts both Postgres and MySQL with predictable settings.

Start the services:

```sh
docker compose up -d
```

This will launch:

- Postgres (container: `test-pavex-session-postgres`)
- MySQL (container: `test-pavex-session-mysql`)

You can check their status with:

```sh
docker compose ps
```

and stop them when done with:

```sh
docker compose down
```

## Running Tests

Once the databases are running, you can run the Rust test suite:

```sh
cargo test
```

The test code is configured to connect to:

- Postgres: `postgres://test:test@localhost:55432/session_test`
- MySQL: `mysql://test:test@localhost:53306/session_test`

## Tips

- If you need to reset the databases, simply run:
  ```sh
  docker compose down -v
  docker compose up -d
  ```
- The containers have health checks configured, so they may take a few seconds before being ready.
  If you see connection errors, wait a few seconds and try again.
