# Error handling

In `UserAgent::extract` you're only handling the happy path:
the method panics if the `User-Agent` header contains characters that are not [ASCII printable](https://en.wikipedia.org/wiki/ASCII#Printable_character_table).
Panicking for bad user input is poor behavior: you should handle the issue gracefully and return an error instead.

Let's change the signature of `UserAgent::extract` to return a `Result` instead:

--8<-- "docs/tutorials/quickstart/snaps/user_agent_fallible_extract.snap"

1. `ToStrError` is the error type by `HeaderValue`'s `to_str` when there is a non-printable ASCII character in the header.

## Error fallbacks

If you try to build the project now, you'll get a warning:

```ansi-color
--8<-- "docs/tutorials/quickstart/snaps/missing_error_handler.snap"
```

You registered a fallible constructor, but there is no [**specific error handler**][error_handler] for its error type.
As it stands, Pavex will invoke the [fallback error handler](/guide/errors/error_handlers.md#fallback-error-handler) when
the constructor fails, returning a generic `500 Internal Server Error` response.

Let's handle `ToStrError` properly.

## Add an error handler

An [error handler][error_handler] must convert a reference to the error type into a [`Response`][Response][^dependency_injection].

Error handlers decouple the detection of an error from its representation on the wire: a constructor doesn't need to know how the
error will be represented in the response, it just needs to signal that something went wrong.
You can then change the representation of an error on the wire without touching the constructor: you only need to change
the error handler.

Define a new `invalid_user_agent` function in `app/src/user_agent.rs`:

--8<-- "docs/tutorials/quickstart/snaps/invalid_user_agent.snap"

The application should compile successfully now.

[Blueprint]: /api_reference/pavex/struct.Blueprint.html
[Response]: /api_reference/pavex/struct.Response.html
[error_handler]: /guide/errors/error_handlers.md

[^dependency_injection]: Error handlers, just like routes and constructors, can take advantage of dependency injection!
    You could, for example, change the response representation according to the `Accept` header specified in the request.
