# Installation

## Prerequisites

To work with Pavex, you'll need:

- [The Rust toolchain](https://www.rust-lang.org/). In particular:
    - `rustup`, the Rust toolchain manager. Check out the [installation instructions](https://rustup.rs/).
    - `cargo`, the Rust package manager. It's automatically installed when installing `rustup`.
- [`cargo-px`](https://github.com/LukeMathWalker/cargo-px), a `cargo` subcommand. You can install it by running:
  ```bash
  cargo install --locked cargo-px --version="~0.1"
  ```
  
All these tools need to be available in your `PATH`.  
If you're not sure whether that's the case, you can check by running:
```bash
rustup --version && \
  cargo --version && \
  cargo px --version
```

If there are no errors, you're good to go!

## Pavex

Pavex provides a command-line interface to scaffold new projects and work with existing ones.  
To install it, execute the following command:

```bash
cargo install --locked \
  --git "https://github.com/LukeMathWalker/pavex.git" \
  --branch "main" \
  pavex_cli
```

You can check that it's been installed correctly by running:

```bash
pavex --version
```

If there are no errors, you're ready to [embark on your Pavex journey](learning_paths.md)!