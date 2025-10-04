# Testing

HTTP sessions are security-critical components of a backend application.\
It is therefore important to test the session management system thoroughly.

Whenever you submit a pull request that changes the session management system,\
you must include tests that verify the correctness of the changes.

There are two key tools you can rely on to spot if more tests are needed:

- **Code coverage**:\
  The code coverage tool will show you which parts of the code are not tested.\
  If you see that some parts of the session management system are not tested,\
  you should write tests for them.\
  Use [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) to generate a code coverage report:

  ```bash
  cargo llvm-cov --no-cfg-coverage -p pavex_session --html --open
  ```
- **Mutation testing**:\
  Mutation testing is a technique to evaluate the quality of your tests.\
  It works by introducing small changes (mutations) to the code and checking if the tests catch them.\
  If the tests do not catch the mutations, it means that the tests are not thorough enough.\
  Use [`cargo-mutants`](https://mutants.rs/welcome.html) to run mutation tests:

  ```bash
  # Exclude Debug and Display implementations from mutation testing
  cargo mutants -E "impl Debug" -E "impl Display" -p pavex_session
  ```
