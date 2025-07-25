# Errors

[Routes](../routing/index.md),
[constructors](../dependency_injection/constructors.md), [middlewares](../middleware/index.md):
they can **fail**.

--8<-- "docs/examples/errors/error_handlers/fallible.snap"

1. The request handler is fallible because it returns a `Result`, with `LoginError` as its error type.

What happens on failure? What should the framework do with the error?\
Two different concerns must be addressed:

- **Reacting**: whoever called your API is waiting for a response! The error must be converted into an HTTP response.
- **Reporting**: you need to know when something goes wrong—and why.\
  You must be able to _report_ that an error occurred using your preferred monitoring system (e.g.
  a log record, incrementing a counter, sending a notification, etc.).

These concerns are addressed by two different kinds of Pavex components: [**error handlers**](error_handlers.md)
and [**error observers**](error_observers.md).

!!! note

    Check out [this article](https://www.lpalmieri.com/posts/error-handling-rust/) for a deep dive 
    on the topic of error handling (in Rust and beyond).
