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

- [`rustup`](https://rustup.rs/)
- [`cargo-px`](https://lukemathwalker.github.io/cargo-px/)

# Running tests

```bash
cargo build_compiler && cargo test
```

We primarily rely on end-to-end testing to check that Pavex's behaviour meets our expectations. We refer to these end-to-end tests
as **UI tests**.\
The UI test suite is attached to the `pavex_cli` crate and relies on a custom test harness, which you can find at `/compiler/pavex_test_runner`. The actual UI tests are found under `compiler/ui_tests`.

In a nutshell:

- each test needs to live in its own folder;
- each test must include a `test_config.toml` explaining what the test is about and/or configuring expectations;
- all testing is snapshot-based and the expected outcomes must be provided in an `expectations` sub-folder;
- if the test is expected to pass, we check the generated code and the graph diagnostics;
- if the test is expected to fail, we check `stderr` to verify the quality of the error message returned to users.

## Updating saved snapshots

The generated code or the graph diagnostics may not match our expectations.\
The test runner will save the unexpected output in a file named like the expectation file with an additional `.snap` suffix. You can then choose to update the saved snapshot via our utility CLI:

```bash
# It must be run from the root of the repository
cargo r --bin snaps
```

It will cycle through all `.snap` files and print the changeset with respect to our previous expectations.\
You will then be prompted to decide if you want to update the saved snapshot to match the new value or if you prefer to keep it as it.

# Updating code examples in the documentation

Most snippets in the documentation hosted on [pavex.dev](https://pavex.dev/docs) are **automatically generated**.

In the documentation file, you'll see an include directive that looks like this:

```markdown
--8<-- "docs/examples/quickstart/06-extract.snap"
```

The path is relative to the root of the repository.\
**Do not modify `*.snap` files directly**.
If you do, the `is-up-to-date` CI check will fail when you open a pull request.

## `pxh`

All snippets are extracted using the [`pxh` binary](docs/tools/pxh).
To work on docs, start by installing it:

```bash
cargo install --path docs/tools/pxh
```

Then install the `pavexc` binary to make sure that any change you made locally is picked up:

```bash
cargo install -p pavexc_cli
```

Then, to regenerate the snippets:

```bash
cd docs/examples
pxh example regenerate
```

You can also choose to regenerate the snippet for a subset of the documentation. E.g. to regenerate the snippets for the attributes chapter:

```bash
cd docs/examples/attributes
pxh example regenerate
```
