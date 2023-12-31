# Path patterns

A **path pattern** is a string that determines which requests are matched by a given route based on their path.

## Static paths

The simplest case is a static path, a path pattern that matches a single, fixed path:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/path_patterns/static/src/blueprint.rs"
```

It will only match requests with a path that is **exactly equal** to `/greet`.

## Route parameters

Static paths are fairly limited. The real power of path patterns comes from their ability to match **dynamic paths**:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/path_patterns/named_parameter/src/blueprint.rs"
```

The `:name` segment is a **route parameter**.  
It matches everything after `/greet/`, up to the next `/` or the end of the path.  
It matches, for example, `/greet/Ursula` and `/greet/John`. It won't match `/greet/` though!

You can have multiple route parameters in a single path pattern, as long as they are separated by a static segment:

```rust hl_lines="8"
--8<-- "doc_examples/guide/routing/path_patterns/multi_named_parameter/src/blueprint.rs"
```

## Catch-all parameters

Route parameters prefixed with a `:` only match a single path segment—they stop at the next `/` or at the end of the path.  
You can use the `*` character to craft a **catch-all** route parameter. It matches the rest of the path, regardless of its contents:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/path_patterns/catch_all_parameter/src/blueprint.rs"
```

`*details` matches everything after `/greet/`, even if it contains `/` characters.
`/greet/*details` matches, for example, `/greet/Ursula` and `/greet/John`, but it also matches `/greet/Ursula/Smith` and `/greet/John/Doe`.

To avoid ambiguity,
you can have **at most one catch-all parameter per path pattern** and it must be **at the end of the path pattern**.

## Accessing route parameters

Route parameters are not discarded after a request has been routed.
You can access their values from your request handler or from middlewares.

Check out the ["Route parameters"](../request_data/path/route_parameters.md) guide for more details.


[RouteParams]: ../../api_reference/pavex/request/route/struct.RouteParams.html
