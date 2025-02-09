# Installation

## Pavex CLI

To work on a Pavex project you need its command-line interface, `pavex`.  
Execute one of the following commands to install it:

=== "shell"

    ```bash
    curl --proto '=https' --tlsv1.2 -LsSf https://pavex.dev/install.sh | sh
    ```

=== "powershell"

    ```powershell
    powershell -c "irm https://pavex.dev/install.ps1 | iex"
    ```

## Activation and setup

Before you can start using `pavex`, you need to activate it and install its dependencies.
Visit [console.pavex.dev](https://console.pavex.dev) and follow the instructions there to complete the activation process.

If there are no errors, you're ready to [embark on your Pavex journey](learning_paths.md)!

??? "Pavex's dependencies"

    Pavex needs:

    - [`rustup`](https://rustup.rs/), Rust's toolchain manager
    - `cargo`, Rust's package manager
    - [`cargo-px`](https://github.com/LukeMathWalker/cargo-px), a `cargo` subcommand
    - A specific Rust's nightly toolchain with the [`rustdoc-json`](https://github.com/rust-lang/rust/issues/76578) component

    `rustup` and `cargo` must be available in your `PATH`.

    On nightly: **Pavex will never use the nightly toolchain to compile your application**.  
    All the code you'll be running (in production or otherwise) will be compiled with the stable toolchain.
    Pavex relies on `nightly` to perform code generation and compile-time reflection—nothing else.

### Verifying your setup

At any point in time, you can invoke `pavex self setup` to verify that all the necessary dependencies are installed 
on your system and that they are configured as Pavex expects them to.  
It's a good first step if you're experiencing issues with your setup.
