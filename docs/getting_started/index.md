# Installation

## Prerequisites

To work with Pavex, you'll need:

- [The Rust toolchain](https://www.rust-lang.org/). In particular:
    - `rustup`, the Rust toolchain manager. Check out its [installation instructions](https://rustup.rs/).
    - `cargo`, the Rust package manager. It's automatically installed when installing `rustup`.
- [`cargo-px`](https://github.com/LukeMathWalker/cargo-px), a `cargo` subcommand. Check out its [installation instructions](https://lukemathwalker.github.io/cargo-px/).
  
All these tools need to be available in your `PATH`.  
If you're not sure whether that's the case, you can check by running:
```bash
rustup --version && \
  cargo --version && \
  cargo px --version
```

If there are no errors, you're good to go!

### Nightly toolchain

To perform code generation, Pavex relies on an unstable Rust feature:
[`rustdoc-json`](https://github.com/rust-lang/rust/issues/76578).  
As a consequence, Pavex requires you to have the Rust `nightly` toolchain installed.

You can add `nightly` to your toolchain by running:
```bash
rustup toolchain install nightly
```

Once `nightly` is installed, add the `rust-docs-json` component:

```bash
rustup component add --toolchain nightly rust-docs-json
```

**Pavex will never use `nightly` to compile your application**.  
All the code you'll be running (in production or otherwise) will be compiled with the stable toolchain. 
Pavex relies on `nightly` to perform code generation and compile-time reflection—nothing else.

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