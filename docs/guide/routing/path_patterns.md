# Path patterns

A **path pattern** is a string that determines which requests are matched by a given route based on their path.

## Static paths

The simplest case is a static path, a path pattern that matches a single, fixed path:

--8<-- "docs/examples/routing/core_concepts/static_path.snap"

It will only match requests with a path that is **exactly equal** to `/greet`.

## Path parameters

Static paths are fairly limited. The real power of path patterns comes from their ability to match **dynamic paths**:

--8<-- "docs/examples/routing/core_concepts/named_parameter.snap"

The `{name}` segment is a [**path parameter**](../request_data/path/path_parameters.md).
It matches everything after `/greet/`, up to the next `/` or the end of the path.
It matches, for example, `/greet/Ursula` and `/greet/John`. It won't match `/greet/` though!

You can have multiple path parameters in a single path pattern, as long as they are don't belong to the same segment:

--8<-- "docs/examples/routing/core_concepts/multiple_named_parameters.snap"

## Catch-all parameters

Normal path parameters match a single path segment—they stop at the next `/` or at the end of the path.
You can use the `*` character to craft a **catch-all** path parameter. It matches the rest of the path, regardless of its contents:

--8<-- "docs/examples/routing/core_concepts/catch_all_parameter.snap"

`{*details}` matches everything after `/info/{name}`, even if it contains `/` characters.
`/info/{name}/{*details}` matches, for example, `/info/ursula/le_guin` and `/info/ursula/mc_guire`, but it also matches `/info/ursula/mc_guire/le_guin`.\
It won't match `/info/ursula/mc_guire/le_guin/` though! The matched portion can't be empty.

To avoid ambiguity,
you can have **at most one catch-all parameter per path pattern** and it must be **at the end of the path pattern**.

## Accessing path parameters

Path parameters are not discarded after a request has been routed.
You can access their values from your handler or from middlewares.

Check out the ["Path parameters"](../request_data/path/path_parameters.md) guide for more details.

[PathParams]: /api_reference/pavex/request/path/struct.PathParams.html
