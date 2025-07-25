# demo

# Getting started

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Pavex]
- [`cargo-px`]
- _(Optional but recommended)_ [`cargo-hakari`](https://docs.rs/cargo-hakari/0.9.29/cargo_hakari/index.html#installation)

## Useful commands

This API is built using the [Pavex] web framework, which relies on code generation.\
You need to use the `cargo px` command instead of `cargo`: it ensures that the
`server_sdk` crate is correctly regenerated when the application blueprint changes.

[`cargo-px`] is a wrapper around `cargo` that will automatically regenerate the
server SDK when needed.
Check out its [documentation](https://github.com/LukeMathWalker/cargo-px)
for more details.

### Build

```bash
# You can also use `cargo px b`, if you prefer.
cargo px build
```

### Run

```bash
# You can also use `cargo px r`, if you prefer.
cargo px run
```

The command above will launch the API, which will start listening on
port `8000`.
The API will use the `dev` profile. Check out [CONFIGURATION.md] for more details.

### Test

```bash
# You can also use `cargo px t`, if you prefer.
cargo px test
```

## Configuration

The configuration system used by this application is detailed in [CONFIGURATION.md].

## `workspace_hack`

The project includes a "utility" crate named `workspace_hack`. It is used
to speed up project builds by forcing eager feature unification.\
To understand how it works, check out [`cargo-hakari`'s documentation](https://docs.rs/cargo-hakari/0.9.29/cargo_hakari/about/index.html).\
To keep `workspace_hack` up to date, install `cargo-hakari` and run:

```bash
cargo hakari generate && cargo hakari manage-deps -y
```

[Pavex]: https://pavex.dev
[`cargo-px`]: https://lukemathwalker.github.io/cargo-px/
[CONFIGURATION.md]: CONFIGURATION.md
