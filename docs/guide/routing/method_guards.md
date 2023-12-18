# Method guards

A **method guard** determines which HTTP methods are allowed for a given route.
They are modelled by the [`MethodGuard`][MethodGuard] type.

## Single-method guards

The simplest case is a guard that allows a single HTTP method:

```rust  hl_lines="6"
--8<-- "doc_examples/code_samples/guide/routing/method_guards/single_method/src/blueprint.rs"
```

This is by far the most common case and Pavex provides short-hands for it: in the
[`pavex::blueprint::router`][pavex::blueprint::router#constants] module there is
a pre-built guard for each well-known HTTP method (e.g. `GET` in the example above).

## Multi-method guards

You can build a guard that accepts multiple HTTP methods by combining single-method guards
with the [`or`][or] method:

```rust hl_lines="7"
--8<-- "doc_examples/code_samples/guide/routing/method_guards/multi_method/src/blueprint.rs"
```

## Ignoring the method

If you don't care about the HTTP method of the incoming request, use the [`ANY`][ANY] method guard:

```rust hl_lines="6"
--8<-- "doc_examples/code_samples/guide/routing/method_guards/any/src/blueprint.rs"
```

[`ANY`][ANY] matches all well-known HTTP methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`, `CONNECT` and
`TRACE`.  
It won't match, however, custom HTTP methods (e.g. `FOO`).
If you truly want to match _any_ HTTP method, use [`ANY_WITH_EXTENSIONS`][ANY_WITH_EXTENSIONS] instead.

[MethodGuard]: ../../api_reference/pavex/blueprint/router/struct.MethodGuard.html
[pavex::blueprint::router#constants]: ../../api_reference/pavex/blueprint/router/index.html#constants
[or]: ../../api_reference/pavex/blueprint/router/struct.MethodGuard.html#method.or
[ANY]: ../../api_reference/pavex/blueprint/router/constant.ANY.html
[ANY_WITH_EXTENSIONS]: ../../api_reference/pavex/blueprint/router/constant.ANY_WITH_EXTENSIONS.html
