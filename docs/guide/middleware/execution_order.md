# Execution order

Pavex provides three types of middlewares: [pre-processing], [post-processing], and [wrapping middlewares].\
When all three types of middlewares are present in the same request processing pipeline, it can be challenging to figure
out the order in which they will be executed.\
This guide will help you build a mental model for Pavex's runtime behaviour.

## Same kind

Let's start with the simplest case: all registered middlewares are of the same kind.
The middlewares will be executed in the order they were registered.\
But let's review some concrete examples to make sure we're on the same page.

### Pre-processing

--8<-- "docs/examples/middleware/order/pre_only.snap"

When a request arrives, the following sequence of events will occur:

1. `PRE_1` is invoked and executed to completion.
2. `PRE_2` is invoked and executed to completion.
3. `GET_INDEX` is invoked and executed to completion.

If `PRE_1` returns an early response, the rest of the request processing pipeline will be skipped—i.e.
`PRE_2` and `GET_INDEX` will not be executed.

### Post-processing

--8<-- "docs/examples/middleware/order/post_only.snap"

When a request arrives, the following sequence of events will occur:

1. `GET_INDEX` is invoked and executed to completion.
2. `POST_1` is invoked and executed to completion.
3. `POST_2` is invoked and executed to completion.

### Wrapping

--8<-- "docs/examples/middleware/order/wrap_only.snap"

When a request arrives, the following sequence of events will occur:

1. `WRAP_1` is invoked.
   1. `next.await` is called inside `WRAP_1`
      1. `WRAP_2` is invoked.
         1. `next.await` is called inside `WRAP_2`
            1. `GET_INDEX` is invoked and executed to completion.
         2. `WRAP_2` completes.
   2. `WRAP_1` completes.

## Different kinds

Let's now consider more complex scenarios: we have multiple kinds of middlewares in the same request processing pipeline.

### Pre- and post-, no early return

Let's start with a scenario where pre-processing and post-processing middlewares are present in the same request processing pipeline.

--8<-- "docs/examples/middleware/order/pre_and_post.snap"

When a request arrives, the following sequence of events will occur:

1. `PRE_1` is invoked and executed to completion.
2. `PRE_2` is invoked and executed to completion.
3. `GET_INDEX` is invoked and executed to completion.
4. `POST_1` is invoked and executed to completion.
5. `POST_2` is invoked and executed to completion.

Pavex doesn't care about the fact that `POST_1` was registered before `PRE_1`.\
Pre-processing middlewares are guaranteed to be executed before the request handler,
and post-processing middlewares are guaranteed to be executed after the request handler.
As a consequence, pre-processing middlewares will always be executed before post-processing middlewares.

Pavex relies on registration order as a way to sort middlewares of the same kind.

### Pre- and post-, early return

Let's consider the same scenario we had above:

--8<-- "docs/examples/middleware/order/pre_and_post.snap"

This time we'll assume that `PRE_1` returns [`Processing::EarlyReturn`][Processing::EarlyReturn]
instead of [`Processing::Continue`][Processing::Continue].
The following sequence of events will occur:

1. `PRE_1` is invoked and returns [`Processing::EarlyReturn`][Processing::EarlyReturn].
2. `PRE_2` is **skipped**.
3. `GET_INDEX` is **skipped**.
4. `POST_1` is invoked and executed to completion.
5. `POST_2` is invoked and executed to completion.

Post-processing middlewares are still invoked, even if the request processing pipeline is interrupted by an early return.

### Pre- and wrapping

Let's now consider a scenario where pre-processing and wrapping middlewares are present in the same request processing pipeline.

--8<-- "docs/examples/middleware/order/pre_and_wrap.snap"

When a request arrives, the following sequence of events will occur:

1. `PRE_1` is invoked and executed to completion.
2. `WRAP_1` is invoked.
   1. `next.await` is called inside `WRAP_1`
      1. `PRE_2` is invoked and executed to completion.
      2. `WRAP_2` is invoked.
         1. `next.await` is called inside `WRAP_2`
            1. `PRE_3` is invoked and executed to completion.
            2. `GET_INDEX` is invoked and executed to completion.
         2. `WRAP_2` completes.
   2. `WRAP_1` completes.

Pre-processing and wrapping middlewares can be **interleaved**, therefore their execution order
matches the order in which they were registered.

If `PRE_2` returns an early response, the rest of the request processing pipeline will be skipped—i.e.
`WRAP_2`, `PRE_3` and `GET_INDEX` will not be executed.
`WRAP_1` will execute to completion, since it was already executing when `PRE_2` was invoked.
In particular, `next.await` in `WRAP_1` will return the early response chosen by `PRE_2`.

### Post- and wrapping

Let's now consider a scenario where post-processing and wrapping middlewares are present in the same request processing pipeline.

--8<-- "docs/examples/middleware/order/post_and_wrap.snap"

When a request arrives, the following sequence of events will occur:

1. `WRAP_1` is invoked.
   1. `next.await` is called inside `WRAP_1`
      1. `GET_INDEX` is invoked and executed to completion.
      2. `POST_2` is invoked and executed to completion.
   2. `WRAP_1` completes.
2. `POST_1` is invoked and executed to completion.

Wrapping middlewares must begin their execution before the request handler, therefore they will always be executed
before post-processing middlewares.\
Registration order matters the way out, though: `WRAP_1` was registered before `POST_2`, therefore `POST_2` will be part
of the request processing pipeline that `WRAP_1` wraps around, i.e. it will be invoked by `next.await`.\
`POST_1`, on the other hand, was registered before `WRAP_1`, therefore it will be invoked after `WRAP_1` completes.

!!! warning

    Wrapping middlewares act as a **reordering boundary**.  
    Even though `POST_1` was registered before `POST_2`, `POST_2` will be executed before `POST_1` in the example above
    since `POST_2` is "captured" inside `WRAP_1`'s scope.

### Pre-, post-, and wrapping

At last, let's examine a scenario where all three types of middlewares are present in the same request processing pipeline.

--8<-- "docs/examples/middleware/order/registration.snap"

If there are no errors or early returns, the following sequence of events will occur:

1. `PRE_1` is invoked and executed to completion.
2. `WRAP_1` is invoked.
   1. `next.await` is called inside `WRAP_1`
      1. `PRE_2` is invoked and executed to completion.
      2. `GET_INDEX` is invoked and executed to completion.
      3. `POST_2` is invoked and executed to completion.
   2. `WRAP_1` completes.
3. `POST_1` is invoked and executed to completion.

### Pre-, post-, and wrapping, early return

Let's consider the same scenario we had above:

--8<-- "docs/examples/middleware/order/registration.snap"

This time, we'll assume that `PRE_1` returns [`Processing::EarlyReturn`][Processing::EarlyReturn].
The following sequence of events will occur:

1. `PRE_1` is invoked and returns [`Processing::EarlyReturn`][Processing::EarlyReturn].
2. `WRAP_1` is **skipped**.
   1. `next.await` is **not** called inside `WRAP_1`
      1. `PRE_2` is **skipped**.
      2. `GET_INDEX` is **skipped**.
      3. `POST_2` is **skipped**.
3. `POST_1` is invoked and executed to completion.

Pay attention to the fact that `POST_2` is not executed, even though it is a post-processing middleware.
That's because of `WRAP_1`: since `POST_2` is part of the request processing pipeline that `WRAP_1` wraps around,
it will be skipped if `WRAP_1` is skipped.

[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[nest]: /api_reference/pavex/blueprint/struct.Blueprint.html#method.nest
[pre-processing]: pre_processing.md
[post-processing]: post_processing.md
[wrapping middlewares]: wrapping.md
[Processing::Continue]: /api_reference/pavex/middleware/enum.Processing.html#variant.Continue
[Processing::EarlyReturn]: /api_reference/pavex/middleware/enum.Processing.html#variant.EarlyReturn
