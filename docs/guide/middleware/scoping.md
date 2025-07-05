# Scoping

Middlewares apply to all routes that were **registered after** them.

--8<-- "docs/examples/middleware/order/mw_after_handler.snap"

The request handler for `GET /` has been registered **before** `WRAP_2`, so it is not affected by it.

## Nesting

The same principle applies to nested [`Blueprint`s][Blueprint].
Middlewares apply to all routes in nested [`Blueprint`s][Blueprint] that were **nested after** the middleware.

--8<-- "docs/examples/middleware/order/mw_after_nested.snap"

The `WRAP_2` middleware has been registered **after** the call to [`.nest`][nest],
so it won't apply to the route registered against the nested [`Blueprint`][Blueprint], `GET /`.

[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[nest]: /api_reference/pavex/blueprint/struct.Blueprint.html#method.nest
