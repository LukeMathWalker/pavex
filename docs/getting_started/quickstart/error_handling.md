# Error handling

In `UserAgent::extract` you're only handling the happy path:
the method panics if the `User-Agent` header contains characters that are not [ASCII printable](https://en.wikipedia.org/wiki/ASCII#Printable_character_table).
Panicking for bad user input is poor behavior: you should handle the issue gracefully and return an error instead.

Let's change the signature of `UserAgent::extract` to return a `Result` instead:

--8<-- "docs/tutorials/quickstart/snaps/user_agent_fallible_extract.snap"

1. `ToStrError` is the error type by `HeaderValue`'s `to_str` when there is a non-printable ASCII character in the header.

## All errors must be handled

If you try to build the project now, you'll get an error from Pavex:

```ansi-color
--8<-- "docs/tutorials/quickstart/snaps/missing_error_handler.snap"
```

Pavex is complaining: you can register a fallible constructor, but you must also register an
[**error handler**](../../guide/errors/error_handlers.md) for it.

## Add an error handler

An error handler must convert a reference to the error type into a [`Response`][Response] (1).\
It decouples the detection of an error from its representation on the wire: a constructor doesn't need to know how the
error will be represented in the response, it just needs to signal that something went wrong.
You can then change the representation of an error on the wire without touching the constructor: you only need to change
the
error handler.
{ .annotate }

1. Error handlers, just like routes and constructors, can take advantage of dependency injection!
   You could, for example, change the response representation according to the `Accept` header specified in the request.

Define a new `invalid_user_agent` function in `app/src/user_agent.rs`:

--8<-- "docs/tutorials/quickstart/snaps/invalid_user_agent.snap"

The application should compile successfully now.

[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[Response]: /api_reference/pavex/response/struct.Response.html
