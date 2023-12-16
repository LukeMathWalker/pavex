# Basics

All **routes** in Pavex must be registered with your application [`Blueprint`][Blueprint] via
its [`route`][Blueprint::route] method:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/basics/src/blueprint.rs"
```

[`Blueprint::route`][Blueprint::route] expects three arguments: a **method guard**, a **path pattern** and a **request
handler**.

## Method guards

A **method guard** determines which HTTP methods are allowed for a given route.
They are modelled by the [`MethodGuard`][MethodGuard] type.  

### Single-method guards

The simplest case is a guard that allows a single HTTP method:

```rust  hl_lines="6"
--8<-- "doc_examples/guide/routing/basics/src/blueprint.rs"
```

This is by far the most common case and Pavex provides short-hands for it: in the
[`pavex::blueprint::router`][pavex::blueprint::router#constants] module there is
a pre-built guard for each well-known HTTP method (e.g. `GET` in the example above).

### Multi-method guards

You can build a guard that accepts multiple HTTP methods by combining single-method guards
with the [`or`][or] method:

```rust hl_lines="7"
--8<-- "doc_examples/guide/routing/basics/src/multi_blueprint.rs"
```

### Ignoring the method

If you don't care about the HTTP method of the incoming request, use the [`ANY`][ANY] method guard:

```rust hl_lines="7"
--8<-- "doc_examples/guide/routing/basics/src/any_method_blueprint.rs"
```

[`ANY`][ANY] matches all well-known HTTP methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`, `CONNECT` and
`TRACE`.  
It won't match, however, custom HTTP methods (e.g. `FOO`).
If you truly want to match _any_ HTTP method, use [`ANY_WITH_EXTENSIONS`][ANY_WITH_EXTENSIONS] instead.

## Path patterns

A **path pattern** is a string that determines which requests are matched by a given route based on their path.

### Static paths

The simplest case is a static path, a path pattern that matches a single, fixed path:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/basics/src/blueprint.rs"
```

It will only match requests with a path that is **exactly equal** to `/greet`.

### Route parameters

Static paths are fairly limited. The real power of path patterns comes from their ability to match **dynamic paths**:
    
```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/basics/src/dynamic_path_blueprint.rs"
```

The `:name` segment is a **route parameter**.  
It matches everything after `/greet/`, up to the next `/` or the end of the path.  
It matches, for example, `/greet/Ursula` and `/greet/John`. It won't match `/greet/` though!

You can have multiple route parameters in a single path pattern, as long as they are separated by a static segment:

```rust hl_lines="8"
--8<-- "doc_examples/guide/routing/basics/src/multi_route_parameters_blueprint.rs"
```

### Accessing route parameters

Route parameters are not discarded after a request has been routed.  
You can access their value from your request handler or from a middleware using the [`RouteParams`][RouteParams] extractor.

Check out the API reference for [`RouteParams`][RouteParams] for more details.

### Catch-all parameters

Route parameters prefixed with a `:` only match a single path segment—they stop at the next `/` or at the end of the path.  
You can use the `*` character to craft a **catch-all** route parameter. It matches the rest of the path, regardless of its contents:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/basics/src/catchall_blueprint.rs"
```

`*details` matches everything after `/greet/`, even if it contains `/` characters. 
It matches, for example, `/greet/Ursula` and `/greet/John`, but it also matches `/greet/Ursula/Smith` and `/greet/John/Doe`.

To avoid ambiguity, you can have **at most one catch-all parameter per path pattern**.

[Blueprint]: ../../../api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::route]: ../../../api_reference/pavex/blueprint/struct.Blueprint.html#method.route
[MethodGuard]: ../../../api_reference/pavex/blueprint/router/struct.MethodGuard.html
[pavex::blueprint::router#constants]: ../../../api_reference/pavex/blueprint/router/index.html#constants
[or]: ../../../api_reference/pavex/blueprint/router/struct.MethodGuard.html#method.or
[ANY]: ../../../api_reference/pavex/blueprint/router/constant.ANY.html
[ANY_WITH_EXTENSIONS]: ../../../api_reference/pavex/blueprint/router/constant.ANY_WITH_EXTENSIONS.html
[RouteParams]: ../../../api_reference/pavex/request/route/struct.RouteParams.html