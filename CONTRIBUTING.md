# Contributing

This project is not open to unsolicited code contributions (for the time being).  
You are more than free to play around with the code though! The instructions below should be enough to get you started. I suggest looking at [`ARCHITECTURE.md`](ARCHITECTURE.md) as well to get a sense of the overall project structure.

# Prerequisites

- Rust's stable toolchain (`rustup toolchain install stable`);
- Rust's nightly toolchain (`rustup toolchain install nightly`);
- `sscache` (see [here](https://github.com/mozilla/sccache#installation) for installation instructions)
- `cargo-px` (`cargo install --locked cargo-px`)

# Running tests

```bash
cargo test 
```

We primarily rely on end-to-end testing to check that Pavex's behaviour meets our expectations.  
All tests are located in `libs/pavex_cli/tests` and are launched using a custom test runner that you can find in `libs/pavex_test_runner`.

In a nutshell:

- each test needs to live in its own folder;
- each test must include a `test_config.toml` explaining what the test is about and/or configuring expectations;
- all testing is snapshot-based and the expected outcomes must be provided in an `expectations` sub-folder;
- if the test is expected to pass, we check the generated code and the graph diagnostics;
- if the test is expected to fail, we check `stderr` to verify the quality of the error message returned to users.

## Test runtime environment

For each test, a runtime environment is created as a sub-folder of `ui_test_envs`, which is in turn generated at the root of Pavex's workspace.  
We use a consistent folder to leverage `cargo` caching and speed up successive test runs. It also allows you to easily inspect the artifacts generated during the test run.  
If you suspect that something funny is going on due to cross-run contamination, delete the `ui_test_envs` folder to get a clean slate.

## Updating saved snapshots

The generated code or the graph diagnostics may not match our expectations.  
The test runner will save the unexpected output in a file named like the expectation file with an additional `.snap` suffix. You can then choose to update the saved snapshot via our utility CLI:

```bash
# It must be run from the root folder of the libs workspace
cargo r --bin snaps
```

It will cycle through all `.snap` files and print the changeset with respect to our previous expectations.  
You will then be prompted to decide if you want to update the saved snapshot to match the new value or if you prefer to keep it as it.

# Checking examples

Each example project under the `examples` folder is its own workspace.  
The easiest way to check that everything compiles and works as expected is to rely on the `./ci.sh` script at root of the repository.  
It runs whatever `cargo` command you specify against all workspaces in this project (i.e. `libs` + all examples).

E.g. `./ci.sh check` will run `cargo check` in each workspace. 

It will also forward any flag to `cargo`—e.g. `./ci.sh check --all-features` will run `cargo check --all-features` in all workspaces.