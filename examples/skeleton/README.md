# Skeleton example

A barebone example that showcases the _mechanics_ of Pavex.  

`app_blueprint` provides two entrypoints:

- The library crate, where the `Blueprint` for the application is built;
- A binary, `bp`, which can be used to generate the server SDK crate.

`app_server_sdk` is code-generated starting from the `Blueprint`, using `cargo-px`.

# Pre-requisites

- `cargo-px` (`cargo install --locked cargo-px`)

# How to build

All commands must be proxied through `cargo-px` in order to re-generate `app_server_sdk` when necessary.  

```bash
# Build the project
cargo px build
# Run tests
cargo px test
# Etc.
```