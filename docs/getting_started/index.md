# Installation

## Pavex CLI

To work on a Pavex project you need its command-line interface, `pavex`.  
Execute one of the following commands to install it:

=== "shell"

    ```bash
    curl --proto '=https' --tlsv1.2 -LsSf https://pavex.dev/releases/download/latest/pavex_cli-installer.sh | sh
    ```

=== "powershell"

    ```powershell
    powershell -c "irm https://pavex.dev/releases/download/latest/pavex_cli-installer.ps1 | iex"
    ```

## Setup

Pavex relies on a few other tools to work as expected. 
Invoke:

```bash
pavex self setup
```

to verify that you have all the necessary dependencies installed on your system.  
If some dependencies are missing, `pavex self setup` will provide instructions on how to install them.

If there are no errors, you're ready to [embark on your Pavex journey](learning_paths.md)!

??? "Pavex's dependencies"

    Pavex needs:

    - [`rustup`](https://rustup.rs/), Rust's toolchain manager
    - `cargo`, Rust's package manager
    - [`cargo-px`](https://github.com/LukeMathWalker/cargo-px), a `cargo` subcommand
    - Rust's nightly toolchain and the [`rustdoc-json`](https://github.com/rust-lang/rust/issues/76578) component

    `rustup` and `cargo` must be available in your `PATH`.

    On nightly: **Pavex will never use the nightly toolchain to compile your application**.  
    All the code you'll be running (in production or otherwise) will be compiled with the stable toolchain.
    Pavex relies on `nightly` to perform code generation and compile-time reflection—nothing else.

### Activation

You'll be asked to provide an **activation key** by `pavex self setup`.  
You can find the activation key for the beta program in the `#activation` channel of Pavex's Discord server.
You can join the waiting list for the beta program on [pavex.dev](https://pavex.dev).

If you need to change your activation key, invoke `pavex self activate`.