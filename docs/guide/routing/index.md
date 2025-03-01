# Routing

All **routes** in Pavex must be registered with the application [`Blueprint`][Blueprint] via
its [`route`][Blueprint::route] method:

```rust hl_lines="6"
--8<-- "doc_examples/guide/routing/core_concepts/src/blueprint.rs"
```

[`Blueprint::route`][Blueprint::route] expects three arguments: a [**method guard**](method_guards.md), a [**path pattern**](path_patterns.md) and a [**request handler**](request_handlers.md).

As your application grows, you can choose to lean into Pavex's more advanced routing features:

- [**Fallbacks**], to customize the response returned when no route matches
- [**Path prefixes**](path_prefixes.md), to reduce repetition in your route definitions
- [**Domain guards**](domain_guards.md), to serve different content based on the domain being requested

[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::route]: /api_reference/pavex/blueprint/struct.Blueprint.html#method.route
[**Fallbacks**]: /api_reference/pavex/blueprint/struct.Blueprint.html#method.fallback
