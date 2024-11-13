# Path prefixes

You can use [`Blueprint::prefix`] to group multiple routes under a common path prefix.

--8<-- "doc_examples/guide/routing/path_prefixes/project-intro.snap"

1. Here we are using a [block expression](https://doc.rust-lang.org/stable/reference/expressions/block-expr.html) 
   since the nested blueprint is particularly short. 
   If it's more complex, you can build it in a separate function, as shown in the next section.

The prefix is prepended to the path of all routes **nested** under it.  
In the example above, we end up with three different route paths:

- `/homes/` and `/homes/{id}`, after applying the `/homes` prefix
- `/`, not influenced by the prefix

## Prefixes are concatenated

You aren't limited to a single level of nesting. You can break down your routes into as many levels as you need—path prefixes
will be concatenated in the order they were declared.

--8<-- "doc_examples/guide/routing/path_prefixes/project-nested.snap"

The `get_room` request handler will be available at `/homes/{home_id}/rooms/{room_id}`, after prepending all relevant prefixes.

## Path parameters are allowed

As shown in the previous example, your path prefixes can contain path parameters.  
There is no difference between a path parameter in a prefix and a path parameter in a route path.

## Restrictions

There are a few restrictions to keep in mind when using path prefixes:

- Prefixes can't be empty.
- Prefixes must start with a `/` character.
- Prefixes must not end with a `/` character.

These constraints are enforced by Pavex at compile time.

## Trailing slashes

Pavex forbids trailing `/` in path prefixes as a safety measure.  
It's easy to accidentally end up with consecutive `/` if a prefix ends with a `/`—e.g. 
`/prefix//path`, using `/prefix/` as prefix and `/path` for your route.  

Since consecutive slashes are rarely desirable, you must add them explicitly to
your route path if that's what you want:

--8<-- "doc_examples/guide/routing/path_prefixes/project-consecutive.snap"

[`Blueprint::prefix`]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.prefix
