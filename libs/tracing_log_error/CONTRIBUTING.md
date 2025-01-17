# Debugging macros

Developing complex `macro_rules!`, such as `log_error!`, can be challenging. 
To help with debugging, consider using these tools:

- The `-Z macro-backtrace` and `-Z trace-macros` unstable compiler flags.
  In particular, `trace-macros` provides a step-by-step expansion of a `macro_rules!` invocation.
  If you're investigating the test invocations, go for:
  ```bash
  RUSTFLAGS="-Z macro-backtrace -Z trace-macros" cargo +nightly test
  ```
  If you're investigating the doc tests, go for:
  ```bash
  RUSTDOCFLAGS="-Z macro-backtrace -Z trace-macros" cargo +nightly test --doc
  ```
  The above is particularly useful if you're trying to understand a macro that's expanding
  to code that doesn't compile or if it fails due to exceeding the compiler recursion limit.
- A [railroad visualization](https://lukaslueg.github.io/macro_railroad_wasm_demo/) of the macro 
  matching rules.
