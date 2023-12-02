# {{crate_name}}

# Getting started

## Prerequisites

- Rust (see [here](https://www.rust-lang.org/tools/install) for instructions)
- `cargo-px`:
  ```bash
  cargo install cargo-px
  ```
- [Pavex's CLI](https://pavex.dev):

## Useful commands

`{{crate_name}}` is built using the [Pavex](https://pavex.dev) web framework, which relies on code generation.  
You need to use the `cargo px` command instead of `cargo`: it ensures that the
`{{crate_name}}_server_sdk` crate is correctly regenerated when the 
application blueprint changes.

`cargo px` is a wrapper around `cargo` that will automatically regenerate the
server SDK when needed. Check out its [documentation](https://github.com/LukeMathWalker/cargo-px)
for more details.

### Build

```bash
cargo px build
```

### Run

```bash
APP_PROFILE=dev cargo px run --bin api
```

### Test

```bash
cargo px test
```

## Configuration

All configuration files are in the `{{crate_name}}_server/configuration` folder.
The settings that are shared across all environments are stored in `{{crate_name}}_server/configuration/base.yml`.

Environment-specific configuration files can be used to override or supply additional values on top the default settings (see `prod.yml`).
You must specify the app profile that you want to use by setting the `APP_PROFILE` environment variable to either `dev`, `test` or `prod`; e.g.:

```bash
APP_PROFILE=prod cargo px run --bin api
```

All configurable parameters are listed in `{{crate_name}}/src/configuration.rs`.
