# Contributing

This project is not open to unsolicited code contributions (for the time being).  
You are more than free to play around with the code though! The instructions below should be enough to get
you started. I suggest looking at [`ARCHITECTURE.md`](ARCHITECTURE.md) as well to get a sense of the overall project
structure.

# Prerequisites

- Rust's stable toolchain (`rustup toolchain install stable`);
- Rust's nightly toolchain (`rustup toolchain install nightly`);
- `sscache` (see [here](https://github.com/mozilla/sccache#installation) for installation instructions)
- `cargo-nextest` (see [here](https://nexte.st/book/installation.html) for installation instructions)

# Running tests

```bash
cargo nextest run
```

We primarily rely on end-to-end testing to check that `pavex`'s behaviour meets our expectations.  
All tests are located in `libs/pavex_cli/tests` and are launched using a custom test runner that you can find
in `libs/pavex_test_runner`.

In a nutshell:

- each test needs to live in its own folder;
- each test must include a `test_config.toml` explaining what the test is about and/or configuring expectations;
- all testing is snapshot-based and the expected outcomes must be provided in an `expectations` sub-folder;
- if the test is expected to pass, we check the generated code and the graph diagnostics;
- if the test is expected to fail, we check `stderr` to verify the quality of the error message returned to users.

For each, a runtime environment is created as a sub-folder of `ui_test_envs`, which is in turn generated at the root
of `pavex`'s workspace.  
We use a consistent folder to leverage `cargo` caching and speed up successive test runs. It also allows you to easily
inspect the artifacts generated during the test run.  
If you suspect that something funny is going on due to cross-run contamination, delete the `ui_test_envs` folder to get
a clean slate.