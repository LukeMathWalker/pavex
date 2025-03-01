# Post-processing

Post-processing middlewares are invoked after the request handler.\
They are suitable for modifying the response and/or performing side-effects based on its contents.
E.g. logging the response's status code, injecting response headers, etc.

--8<-- "doc_examples/guide/middleware/post/project-basic.snap"

## Signature

Pavex accepts a wide range of function signatures for post-processing middlewares. There are two constraints:

- They must take a [`Response`][Response] as one of their input parameters.
- They must return a [type that can be converted into a `Response` via the `IntoResponse` trait](#intoresponse).\
  If fallible, [they can return a `Result` with a type that implements `IntoResponse` as its `Ok` variant](#middlewares-can-fail).

Other than that, you have a lot of freedom in how you define your post-processing middlewares:

- They can be free functions or methods.
- They can be synchronous or asynchronous.
- [They can take additional input parameters, leaning on Pavex's dependency injection system](#dependency-injection).
- [If fallible, they can use whatever error type you prefer, as long as you provide an error handler for it](#middlewares-can-fail).

The next sections of this guide will elaborate on each of these points.

## Registration

You register a post-processing middleware against a blueprint via the [`post_process`](crate::blueprint::Blueprint::post_process) method.

--8<-- "doc_examples/guide/middleware/post/project-registration.snap"

You must provide an **[unambiguous path]** to the middleware, wrapped in the [`f!`][f] macro.

The middleware will be invoked for all request handlers registered after it, as long as they were registered against the same [`Blueprint`][Blueprint]
or one of its nested children.
Check out the [scoping section](scoping.md) for more details.

!!! note "Registration syntax"

    You can use free functions, static methods, non-static methods, and trait methods as middlewares.
    Check out the [dependency injection cookbook](../dependency_injection/cookbook.md) for more details on
    the syntax for each case.

## `IntoResponse`

Post-processing middlewares, like request handlers, must return a type that can be converted into a [`Response`][Response] via the
[`IntoResponse`][IntoResponse] trait.\
If you want to return a custom type from your middleware, you must implement [`IntoResponse`][IntoResponse] for it.

## Middlewares can fail

Post-processing middlewares can be fallible, i.e. they can return a [`Result`][Result].

--8<-- "doc_examples/guide/middleware/post/project-fallible.snap"

If they do, you must specify an [**error handler**](../errors/error_handlers.md) when registering them:

--8<-- "doc_examples/guide/middleware/post/project-registration_with_error_handler.snap"

Check out the [error handling guide](../errors/error_handlers.md) for more details.

## Dependency injection

Post-processing middlewares can take advantage of **dependency injection**.

You must specify the dependencies of your middleware as **input parameters** in its function signature.\
Those inputs are going to be built and injected by the framework, according to the **constructors** you have registered.

Post-processing middlewares, like request handlers and pre-processing middlewares,
can **mutate request-scoped types**.
Ask for a `&mut` reference to the type you want to mutate as an input parameter, the framework will take care of the rest.

Check out the [dependency injection guide](../dependency_injection/index.md) for more details
on how the process works.\
Check out the [request data guide](../request_data/index.md) for an overview of the data you can extract from the request
using Pavex's first-party extractors.

[f]: /api_reference/pavex/macro.f.html
[IntoResponse]: /api_reference/pavex/response/trait.IntoResponse.html
[Response]: /api_reference/pavex/response/struct.Response.html
[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[Next]: /api_reference/pavex/middleware/struct.Next.html
[instrument]: https://docs.rs/tracing/0.1.40/tracing/trait.Instrument.html#method.instrument
[timeout]: https://docs.rs/tokio/1.35.1/tokio/time/fn.timeout.html
[Future]: https://doc.rust-lang.org/std/future/trait.Future.html
[IntoFuture]: https://doc.rust-lang.org/std/future/trait.IntoFuture.html
[Result]: https://doc.rust-lang.org/std/result/index.html
[unambiguous path]: ../dependency_injection/cookbook.md#unambiguous-paths
