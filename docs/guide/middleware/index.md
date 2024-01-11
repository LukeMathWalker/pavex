# Middleware

Middlewares are a mechanism to execute logic before and/or after the request handler.  

--8<-- "doc_examples/guide/middleware/core_concepts/project-basic.snap"

Middlewares are often used to implement **cross-cutting functionality**, such as:

- telemetry (e.g. structured logging, metrics, etc.)
- load-shedding (e.g. timeouts, rate-limiting, etc.)
- access control (e.g. authentication, authorization, etc.)
- etc.

## Registration

You register a middleware against a blueprint via the [`wrap`](crate::blueprint::Blueprint::wrap) method.

--8<-- "doc_examples/guide/middleware/core_concepts/project-registration.snap"

When registering a middleware, you must provide its **fully qualified path**, wrapped in the [`f!` macro][f].  
A middleware applies to all request handlers registered against the same [`Blueprint`][Blueprint].
See the [execution order](#execution-order) section for more details.

!!! note "Registration syntax"

    You can use free functions, static methods, non-static methods, and trait methods as middlewares.
    Check out the [dependency injection cookbook](../dependency_injection/cookbook.md) for more details on
    the syntax for each case.


## `IntoResponse`

Middlewares, like request handlers, must return a type that can be converted into a [`Response`][Response] via the
[`IntoResponse`][IntoResponse] trait.  
If you want to return a custom type from your middleware, you must implement [`IntoResponse`][IntoResponse] for it.

## `Next`

Middlewares **wrap** around the rest of the request processing pipeline.
They are invoked before the request handler and _all the other middlewares_ that were registered later. 
The remaining request processing pipeline is represented by the [`Next`][Next] type.  

All middlewares must take an instance of [`Next`][Next] as input.  
To invoke the rest of the request processing pipeline, you call `.await` on the [`Next`][Next] instance.

--8<-- "doc_examples/guide/middleware/core_concepts/project-basic.snap"

You can also choose to go through the intermediate step of converting [`Next`][Next] into a [`Future`][Future] via the
[`IntoFuture`][IntoFuture] trait.  
This can be useful when you need to invoke APIs that _wrap_ around a [`Future`][Future] (e.g. [`tokio::time::timeout`][timeout]
for timeouts or `tracing`'s [`.instrument()`][instrument] for logging).

--8<-- "doc_examples/guide/middleware/core_concepts/project-into_future.snap"

## Dependency injection

Middlewares can take advantage of **dependency injection**.

You must specify the dependencies of your middleware as **input parameters** in its function signature.  
Those inputs are going to be built and injected by the framework, according to the **constructors** you have registered.

Check out the [dependency injection guide](../dependency_injection/index.md) for more details
on how the process works.  
Check out the [request data guide](../request_data/index.md) for an overview of the data you can extract from the request
using Pavex's first-party extractors.

## Middlewares can fail

Your middlewares can be fallible, i.e. they can return a [`Result`][Result].

--8<-- "doc_examples/guide/middleware/core_concepts/project-fallible.snap"

If they do, you must specify an **error handler** when registering them:

--8<-- "doc_examples/guide/middleware/core_concepts/project-registration_with_error_handler.snap"

The error handler is in charge of building a response from the error returned by the middleware. Just like
a middleware:

- It can be synchronous or asynchronous.
- It can take advantage of dependency injection.
- It must return a type that implements [`IntoResponse`][IntoResponse].

In addition, it must take a reference to the error type as one of its input parameters:

--8<-- "doc_examples/guide/middleware/core_concepts/project-error_handler.snap"

## Execution order

Middlewares are executed in the order they are registered. 

### Example

Let's consider the following request handler and middlewares:

--8<-- "doc_examples/guide/middleware/core_concepts/project-signalers.snap"

Each middleware prints a message before and after invoking the rest of the request processing pipeline.  
The request handler prints a message when it is invoked, before returning a response.

If you register them as follows

--8<-- "doc_examples/guide/middleware/core_concepts/project-vanilla_order.snap"

you'll see this output when you make a request:

```
First - start
Second - start
Handler
Second - end
First - end
```

### Edge cases

Middlewares apply to all routes that were **registered after** them.

--8<-- "doc_examples/guide/middleware/core_concepts/project-mw_after_handler.snap"

1. The route has been registered **before** the `second` middleware, so it is not affected by it.

You'll see this output when you make a request:

```
First - start
Handler
First - end
```

The same principle applies to nested [`Blueprint`s][Blueprint]. 
Middlewares apply to all routes in nested [`Blueprint`s][Blueprint] that were **nested after** the middleware.

--8<-- "doc_examples/guide/middleware/core_concepts/project-mw_after_nested.snap"

1. The nested [`Blueprint`] has been nested **after** the registration of the `first` middleware, so it will apply to its routes.
2. The nested [`Blueprint`] has been nested **before** the registration of the `second` middleware, so it won't apply to its routes.

You'll see this output when you make a request:

```
First - start
Handler
First - end
```

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
