# Wrapping

[Pre-processing](pre_processing.md) and [post-processing](post_processing.md) middlewares can take you a long way, but they can't do everything.
It is impossible, for example, to enforce a request-wide timeout or attach a `tracing` span to the request processing pipeline
using only pre-processing and post-processing middlewares.\
Because of these limitations, Pavex provides a third type of middleware: **wrapping middlewares**.

--8<-- "doc_examples/guide/middleware/wrapping/project-basic.snap"

It's the most powerful kind (although they have [their downsides](#use-with-caution)).\
They let you execute logic before _and_ after the rest of the request processing pipeline.
But, most importantly, they give you access to a future representing the rest of the request processing pipeline
(the [`Next`][Next] type), a prerequisite for those more advanced use cases.

## Registration

You register a wrapping middleware against a blueprint via the [`wrap`](crate::blueprint::Blueprint::wrap) method.

--8<-- "doc_examples/guide/middleware/wrapping/project-registration.snap"

You must provide an **[unambiguous path]** to the middleware, wrapped in the [`f!`][f] macro.

The middleware will be invoked for all request handlers registered after it, as long as they were registered against the same [`Blueprint`][Blueprint]
or one of its nested children.
Check out the [scoping section](scoping.md) for more details.

!!! note "Registration syntax"

    You can use free functions, static methods, non-static methods, and trait methods as middlewares.
    Check out the [dependency injection cookbook](../dependency_injection/cookbook.md) for more details on
    the syntax for each case.

## `IntoResponse`

Wrapping middlewares, like request handlers, must return a type that can be converted into a [`Response`][Response] via the
[`IntoResponse`][IntoResponse] trait.\
If you want to return a custom type from your middleware, you must implement [`IntoResponse`][IntoResponse] for it.

## Middlewares can fail

Wrapping middlewares can be fallible, i.e. they can return a [`Result`][Result].

--8<-- "doc_examples/guide/middleware/wrapping/project-fallible.snap"

If they do, you must specify an [**error handler**](../errors/error_handlers.md) when registering them:

--8<-- "doc_examples/guide/middleware/wrapping/project-registration_with_error_handler.snap"

Check out the [error handling guide](../errors/error_handlers.md) for more details.

## `Next`

Wrapping middlewares **wrap** around the rest of the request processing pipeline.
They are invoked before the request handler and _all the other middlewares_ that were registered later.
The remaining request processing pipeline is represented by the [`Next`][Next] type.

All middlewares must take an instance of [`Next`][Next] as input.\
To invoke the rest of the request processing pipeline, you call `.await` on the [`Next`][Next] instance.

--8<-- "doc_examples/guide/middleware/wrapping/project-basic.snap"

You can also choose to go through the intermediate step of converting [`Next`][Next] into a [`Future`][Future] via the
[`IntoFuture`][IntoFuture] trait.\
This can be useful when you need to invoke APIs that _wrap_ around a [`Future`][Future] (e.g. [`tokio::time::timeout`][timeout]
for timeouts or `tracing`'s [`.instrument()`][instrument] for logging).

--8<-- "doc_examples/guide/middleware/wrapping/project-into_future.snap"

## Dependency injection

Middlewares can take advantage of **dependency injection**.

You must specify the dependencies of your middleware as **input parameters** in its function signature.\
Those inputs are going to be built and injected by the framework, according to the **constructors** you have registered.

Check out the [dependency injection guide](../dependency_injection/index.md) for more details
on how the process works.\
Check out the [request data guide](../request_data/index.md) for an overview of the data you can extract from the request
using Pavex's first-party extractors.

## Use with caution

You should only use wrapping middlewares when you need to access the future representing the rest of the request processing pipeline.
In all other cases, you should prefer pre-processing and post-processing middlewares.

> "But why? Wrapping middlewares can do everything, why not use them all the time?"

Good question! It's because wrapping middlewares and Rust's borrow checker are an explosive combination.

Every time you inject a reference as an input parameter to a wrapping middleware, you are borrowing that reference
for **the whole duration** of the downstream request processing pipeline.
This can easily lead to borrow checker errors, especially if you are working with request-scoped dependencies.
Let's unpack what that means with an example.

### Example

Consider this scenario: you registered a constructor for `MyType`, a request-scoped dependency.
You also registered a wrapping middleware that takes `&MyType` as an input parameter.
You now want to work with `MyType` in your request handler:

- If the request handler takes `&mut MyType` as an input parameter, you'll get an error:
  the immutable reference to `MyType` borrowed by the wrapping middleware is still alive when the request handler is executed.
- If the request handler takes `MyType` by value, Pavex is forced to clone the value to satisfy the borrow checker.
  That's inefficient. If `MyType` isn't clonable, you'll get an error.
- If the request handler takes `&MyType` as an input parameter, all is well. You can have as many immutable references to `MyType` as you want.

You wouldn't have these problems with pre-processing or post-processing middlewares: whatever you inject into them is going to be borrowed
_only_ while the middleware is executed.
You are then free to work with those types in your request handlers/other middlewares as you please.

### No `&mut` references

The scenario we explored above is why Pavex doesn't let you mutate request-scoped types in wrapping middlewares,
a restriction that doesn't apply to request handlers, pre-processing and post-processing middlewares.\
It's so easy to shoot yourself in the foot that it's better to avoid `&mut` references altogether in wrapping middlewares.

[f]: ../../api_reference/pavex/macro.f.html
[IntoResponse]: ../../api_reference/pavex/response/trait.IntoResponse.html
[Response]: ../../api_reference/pavex/response/struct.Response.html
[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[Next]: ../../api_reference/pavex/middleware/struct.Next.html
[instrument]: https://docs.rs/tracing/0.1.40/tracing/trait.Instrument.html#method.instrument
[timeout]: https://docs.rs/tokio/1.35.1/tokio/time/fn.timeout.html
[Future]: https://doc.rust-lang.org/std/future/trait.Future.html
[IntoFuture]: https://doc.rust-lang.org/std/future/trait.IntoFuture.html
[Result]: https://doc.rust-lang.org/std/result/index.html
[unambiguous path]: ../dependency_injection/cookbook.md#unambiguous-paths
