# Core concepts

All **routes** in Pavex must be registered with your application [`Blueprint`][Blueprint] via
its [`route`][Blueprint::route] method:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/basics/src/blueprint.rs"
```

[`Blueprint::route`][Blueprint::route] expects three arguments: a [**method guard**](method_guards.md), a [**path pattern**](path_patterns.md) and a [**request
handler**](request_handlers.md).

[Blueprint]: ../../../api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::route]: ../../../api_reference/pavex/blueprint/struct.Blueprint.html#method.route
