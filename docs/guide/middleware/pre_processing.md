# Pre-processing

Pre-processing middlewares execute ahead of the request handler.  
They can be used to enforce pre-conditions and return an early response if they are not met, 
skipping the rest of the request processing pipeline.
E.g., rejecting unauthenticated requests or enforcing rate limits.

--8<-- "doc_examples/guide/middleware/pre/project-basic.snap"

## Registration

You register a pre-processing middleware against a blueprint via the [`pre_process`](crate::blueprint::Blueprint::pre_process) method.

--8<-- "doc_examples/guide/middleware/pre/project-registration.snap"

You must provide an **[unambiguous path]** to the middleware, wrapped in the [`f!`][f] macro.  

The middleware will be invoked for all request handlers registered after it, as long as they were registered against the same [`Blueprint`][Blueprint]
or one of its nested children.
Check out the [scoping section](scoping.md) for more details.

!!! note "Registration syntax"

    You can use free functions, static methods, non-static methods, and trait methods as middlewares.
    Check out the [dependency injection cookbook](../dependency_injection/cookbook.md) for more details on
    the syntax for each case.


## `Processing`

Pre-processing middlewares must return [`Processing`][Processing].  

--8<-- "doc_examples/guide/middleware/pre/project-basic.snap"

[`Processing`][Processing] is an enum with two variants:

- [`Processing::Continue`][Processing::Continue]: the middleware has finished its work and the request processing pipeline should continue as usual.
- [`Processing::EarlyReturn(T)`][Processing::EarlyReturn]: the remaining pre-processing and wrapping middlewares won't be invoked, nor will the request handler. 
  `T` will be converted into a response, fed to the [post-processing middlewares][post-processing] (if there are any) and then sent back to the client.
  
Check out the [execution order guide](execution_order.md#pre-and-post-early-return) for more details on how the different types of middlewares interact
with each other.

`T`, the type inside [`Processing::EarlyReturn`][Processing::EarlyReturn], must implement the
[`IntoResponse`][IntoResponse] trait.
If `T` is not specified, it defaults to [`Response`][Response].


## Middlewares can fail

Your pre-processing middlewares can be fallible, i.e. they can return a [`Result`][Result].

--8<-- "doc_examples/guide/middleware/pre/project-fallible.snap"

If they do, you must specify an [**error handler**](../errors/error_handlers.md) when registering them:

--8<-- "doc_examples/guide/middleware/pre/project-registration_with_error_handler.snap"

Check out the [error handling guide](../errors/error_handlers.md) for more details.

### `Result` or `Processing::EarlyReturn`?

The rest of the request processing pipeline will be skipped if your pre-processing middleware returns an error.

> Why does `Processing::EarlyReturn` exist then? Can't I just return an error when I want to skip the rest of the pipeline?

You can, but an error has a different **semantic meaning**.  
An error is a **problem** that occurred during the processing of the request.
Pavex will invoke [error observers](../errors/error_observers.md), if they were registered, and your application will
probably emit error-level logs, increment error counters, etc.

There are scenarios where you want to return an early response, but it's not an error.  
E.g., you might want to redirect all requests with a trailing slash to the same URL without the trailing slash.  
An early return is a **normal** response, not an error.

Choose the short-circuiting mechanism that best fits the semantics of your use case.

## Dependency injection

Pre-processing middlewares can take advantage of **dependency injection**.

You must specify the dependencies of your middleware as **input parameters** in its function signature.  
Those inputs are going to be built and injected by the framework,
according to the **constructors** you have registered.  

Pre-processing middlewares, like request handlers and post-processing middlewares,
can **mutate request-scoped types**.
Ask for a `&mut` reference to the type you want to mutate as an input parameter, the framework will take care of the rest.

Check out the [dependency injection guide](../dependency_injection/index.md) for more details
on how the process works.  
Check out the [request data guide](../request_data/index.md) for an overview of the data you can extract from the request
using Pavex's first-party extractors.

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
[Processing]: ../../api_reference/pavex/middleware/enum.Processing.html
[Processing::Continue]: ../../api_reference/pavex/middleware/enum.Processing.html#variant.Continue
[Processing::EarlyReturn]: ../../api_reference/pavex/middleware/enum.Processing.html#variant.EarlyReturn
[post-processing]: post_processing.md
