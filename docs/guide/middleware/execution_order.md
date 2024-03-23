# Execution order

Wrapping middlewares and pre-processing middlewares are executed in registration order.  
Post-processing middlewares are executed in **reverse registration order**.  

## Example

Let's consider this [`Blueprint`][Blueprint]:

--8<-- "doc_examples/guide/middleware/order/project-registration.snap"

If there are no errors or early returns, the following sequence of events will occur:

1. `pre1`, the first pre-processing middleware, is invoked and executed to completion.
2. `wrap1`, the wrapping middleware, is invoked.
    1. `next.await` is called inside `wrap1`, triggering the rest of the request processing pipeline.
        1. `pre2`, the second pre-processing middleware, is invoked and executed to completion.
        2. `handler`, the request handler, is invoked and executed to completion.
        3. `post2`, the second post-processing middleware, is invoked and executed to completion.
    2. `wrap1` completes.
3. `post1`, the first post-processing middleware, is invoked and executed to completion.

## Scoping

Middlewares apply to all routes that were **registered after** them.

--8<-- "doc_examples/guide/middleware/order/project-mw_after_handler.snap"

The request handler for `GET /` has been registered **before** `wrap2`, so it is not affected by it.

The same principle applies to nested [`Blueprint`s][Blueprint].
Middlewares apply to all routes in nested [`Blueprint`s][Blueprint] that were **nested after** the middleware.

--8<-- "doc_examples/guide/middleware/order/project-mw_after_nested.snap"

The `wrap2` middleware has been registered **after** the call to [`.nest`][nest],
so it won't apply to the route registered against the nested [`Blueprint`][Blueprint], `GET /`.

[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[nest]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.nest
