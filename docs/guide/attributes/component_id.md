# Component ids

Pavex requires every component to have a **unique component id**.

## Generation

Attributes will assign a default id to your component based on its name and item kind.

| Item Kind | Naming Scheme               | Example          | Example ID    |
| --------- | --------------------------- | ---------------- | ------------- |
| Function  | `<function name>`           | `my_function()`  | `MY_FUNCTION` |
| Method    | `<type name>_<method name>` | `Span::new()`    | `SPAN_NEW`    |
| Type      | `<type name>`               | `struct MyType;` | `MY_TYPE`     |

The generated id will always be converted to [SCREAMING_SNAKE_CASE][casing].

## Using a custom id

You're not forced to use the default generated id. You can specify a custom one using the `id` argument, supported by all attributes:

--8<-- "docs/examples/attributes/custom_id.snap"

1. The default id would be `AUTH_ERROR_TO_RESPONSE`, but we're using the `id` argument to override it.

## Generated constant

Every attribute will define a new constant next to the annotated component, named after the component's id.
The generated constant can be used to refer to that component when interacting with a [`Blueprint`][Blueprint].

--8<-- "docs/examples/attributes/id_registration.snap"

The generated constants are strongly typed. Your project won't compile if you invoke the wrong [`Blueprint`][Blueprint] method—e.g. `.route()` for an error handler.

## Uniqueness scope

The component id must be **unique within the crate where the component is defined**—e.g. two components with the same identifier from different crates won't cause an issue.

[casing]: https://rust-lang.github.io/api-guidelines/naming.html#casing-conforms-to-rfc-430-c-case
[Blueprint]: /api_reference/pavex/struct.Blueprint.html
