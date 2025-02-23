# Contributing

It's early days and the focus, right now, is on user documentation.\
Nonetheless, the instructions below should be enough to get you going if you choose to submit a pull request!\
I suggest looking at [`ARCHITECTURE.md`](ARCHITECTURE.md) as well to get a sense of the overall project structure.

## CONTRIBUTORS.md

If you choose to contribute to Pavex,
you must add yourself to the [CONTRIBUTORS.md](CONTRIBUTORS.md) file.
It is a lightweight alternative to a full-blown [contributor license agreement](https://www.djangoproject.com/foundation/cla/faq/).

## Unsolicited contributions

Typo in documentation? Open a PR straight-away!\
Small bug fix with a regression test? Open a PR straight-away!\
Anything beyond 20 lines of code? **Open an issue first**.

# Prerequisites

- Rust's stable toolchain (`rustup toolchain install stable`);
- Rust's nightly toolchain (`rustup toolchain install nightly`);
- [`cargo-px`](https://lukemathwalker.github.io/cargo-px/)

# Running tests

```bash
cargo test
```

We primarily rely on end-to-end testing to check that Pavex's behaviour meets our expectations.\
All tests are located in `libs/pavex_cli/tests` and are launched using a custom test runner that you can find in `libs/pavex_test_runner`.

In a nutshell:

- each test needs to live in its own folder;
- each test must include a `test_config.toml` explaining what the test is about and/or configuring expectations;
- all testing is snapshot-based and the expected outcomes must be provided in an `expectations` sub-folder;
- if the test is expected to pass, we check the generated code and the graph diagnostics;
- if the test is expected to fail, we check `stderr` to verify the quality of the error message returned to users.

## Test runtime environment

For each test, a runtime environment is created as a sub-folder of `ui_test_envs`, which is in turn generated at the root of Pavex's workspace.\
We use a consistent folder to leverage `cargo` caching and speed up successive test runs. It also allows you to easily inspect the artifacts generated during the test run.\
If you suspect that something funny is going on due to cross-run contamination, delete the `ui_test_envs` folder to get a clean slate.

## Updating saved snapshots

The generated code or the graph diagnostics may not match our expectations.\
The test runner will save the unexpected output in a file named like the expectation file with an additional `.snap` suffix. You can then choose to update the saved snapshot via our utility CLI:

```bash
# It must be run from the root folder of the libs workspace
cargo r --bin snaps
```

It will cycle through all `.snap` files and print the changeset with respect to our previous expectations.\
You will then be prompted to decide if you want to update the saved snapshot to match the new value or if you prefer to keep it as it.

# Updating code examples in the documentation

Most snippets in the documentation hosted on [pavex.dev](https://pavex.dev/docs) are **automatically generated**.

In the documentation file, you'll see an include directive that looks like this:

```markdown
--8<-- "doc_examples/quickstart/06-extract.snap"
```

The path is relative to the root of the repository.\
**Do not modify `*.snap` files directly**.
If you do, the `is-up-to-date` CI check will fail when you open a pull request.

## `tutorial_generator`

All snippets are extracted using the `tutorial_generator` binary,
defined in the [`doc_examples/tutorial_generator`](doc_examples/tutorial_generator) folder.\
To work on docs, start by installing it:

```bash
cargo install --path doc_examples/tutorial_generator
```

Then install the `pavexc` binary from path to make sure that any change you made locally is picked up:

```bash
cargo install --path libs/pavexc_cli
```

Then, to regenerate the snippets:

```bash
cd doc_examples
# `PAVEX_PAVEXC` tells the generator to use the locally installed `pavexc` binary
# rather than the one from crates.io
PAVEX_PAVEXC=pavexc tutorial_generator
```

You can also choose to regenerate the snippet for a subset of the documentation. E.g. to regenerate the snippets for the quickstart:

```bash
cd doc_examples/quickstart
PAVEX_PAVEXC=pavexc tutorial_generator
```

## `tutorial.yml`

Each "group" of documentation snippets is identified by a `tutorial.yml` file.\
It specifies:

- the starter code for the group
- `git` patches that should be applied on top of the starter code

At every stage of the commit history for the doc project, you can specify in `tutorial.yml`:

- which snippets should be extracted
- which commands should be run (and what their outcome should be)

For a comprehensive example,
check out the [tutorial.yml](doc_examples/quickstart/tutorial.yml) for the quickstart guide.\
For a reference of the available commands, check out the source code of the `tutorial_generator` binary.
