# Request handlers

A **request handler** is invoked when a request matches on the associated [method guard](method_guards.md) and
[path pattern](path_patterns.md).  
The request handler is in charge of building the response that will be sent back to the client.

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/request_handlers/intro/src/routes.rs"
```

## Registration

When registering a route, you must provide the **[fully qualified path]** to the request handler:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/request_handlers/intro/src/blueprint.rs"
```

The path must be wrapped in the [`f!` macro][f!].  

!!! note "Registration syntax"

    You can use free functions, static methods, non-static methods, and trait methods as request handlers.
    Check out the [dependency injection cookbook](../dependency_injection/cookbook.md) for more details on
    the syntax for each case.

## `IntoResponse`

A request handler must return a type that can be converted into a [`Response`][Response] via the
[`IntoResponse`][IntoResponse] trait.  
You only have three first-party types to choose from:

- [`StatusCode`][StatusCode]
- [`ResponseHead`][ResponseHead]
- [`Response`][Response]

This is an explicit design choice. Pavex implements the [`IntoResponse`][IntoResponse] trait exclusively for types
that don't require the framework to _assume_ or _infer_ a suitable status code.

If you want to return a custom type from your request handler, you must implement [`IntoResponse`][IntoResponse] for it.

## Dependency injection

Request handlers are expected to take into account the incoming request when building the response. How does that
work in Pavex? How do you **extract** data from the request?

You must take advantage of **dependency injection**.

You must specify the dependencies of your request handler as **input parameters** in its function signature.  
Those inputs are going to be built and injected by the framework, according to the **constructors** you have registered.

Check out the [dependency injection guide](../dependency_injection/index.md) for more details
on how the process works.  
Check out the [request data guide](../request_data/index.md) for an overview of the data you can extract from the request
using Pavex's first-party extractors.

## Request handlers can fail

Your request handlers can be fallible, i.e. they can return a [`Result`][Result].

```rust
pub async fn greet(/* ... */) -> Result<Response, GreetError> {
    // ...
}
```

If they do, you must specify an [**error handler**](../errors/error_handlers.md) when registering the route:

```rust hl_lines="7"
--8<-- "doc_examples/guide/routing/request_handlers/error_handler/src/blueprint.rs"
```

Check out the [error handling guide](../errors/error_handlers.md) for more details.

## Sync or async?

Request handlers can either be synchronous or asynchronous:

```rust
--8<-- "doc_examples/guide/routing/request_handlers/sync_or_async/src/routes.rs"
```

There is no difference when registering the route with the [`Blueprint`][Blueprint]:

```rust
--8<-- "doc_examples/guide/routing/request_handlers/sync_or_async/src/blueprint.rs"
```

Be careful with synchronous handlers: they **block the thread** they're running on until they return.  
That's not a concern if you are performing an operation that's **guaranteed** to be fast
(e.g. building a response from static data).
It becomes a problem if you're doing work that's **potentially** slow.
There are two types of work that can be slow:

- I/O operations (e.g. reading from a file, querying a database, etc.)
- CPU-intensive operations (e.g. computing a password hash, parsing a large file, etc.)

As a rule of thumb:

| I/O | CPU-intensive | Handler type | Notes                                                                                                                              |
| --- | --------------|--------------|------------------------------------------------------------------------------------------------------------------------------------|
| Yes | No            | Async        | Use async libraries for the I/O portion. If the I/O interface is synchronous, use [`tokio::task::spawn_blocking`][spawn_blocking]. |
| No  | Yes           | Async        | Use [`tokio::task::spawn_blocking`][spawn_blocking] for the CPU-intensive portion.                                                 |
| Yes | Yes           | Async        | See above.                                                                                                                         |
| No  | No            | Sync         | You can also make it asynchronous, it doesn't matter.                                                                              |

If you want to learn more about what "blocking" means in async Rust, check out [Alice Rhyl's excellent article](https://ryhl.io/blog/async-what-is-blocking/).

[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::route]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.route
[IntoResponse]: ../../api_reference/pavex/response/trait.IntoResponse.html
[StatusCode]: ../../api_reference/pavex/http/struct.StatusCode.html
[Response]: ../../api_reference/pavex/response/struct.Response.html
[ResponseHead]: ../../api_reference/pavex/response/struct.ResponseHead.html
[spawn_blocking]: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
[f!]: ../../api_reference/pavex/macro.f.html
[Result]: https://doc.rust-lang.org/std/result/index.html
[fully qualified path](../dependency_injection/cookbook.md)
