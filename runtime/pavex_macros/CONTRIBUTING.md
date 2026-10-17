# Testing

`pavex_macros` relies on [`trybuild`](https://github.com/dtolnay/trybuild) to verify that Pavex macros:

- Accept the expected input/parameter combinations
- Reject invalid input/parameter combinations

In the latter case, we also verify the returned error message to ensure that
diagnostics are accurate and actionable.

## Test filtering

`trybuild` is a custom test runner and it doesn't obey all `cargo test` built-in options.

For filtering, in particular, use

```bash
cargo t -- ui trybuild=not_a_function.rs
```

to run a subset of the whole UI test suite.
The `trybuild` argument doesn't have to be the filename: you can only specify a fragment of the path (e.g. `from_request`) to run
all tests in a specific sub-directory.

## Overwriting saved snapshots

To overwrite saved snapshots, set the `TRYBUILD=overwrite` environment variable:

```bash
# Overwrite all saved snapshots, if changed
TRYBUILD=overwrite cargo t 
# Overwrite snapshots for a subset of tests
TRYBUILD=overwrite cargo t -- ui trybuild=from_request
```
