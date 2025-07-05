# Attributes

**Attributes** are a core part of Pavex's design. You'll use attributes to define every single Pavex component, from routes to middlewares:

--8<-- "docs/examples/attributes/annotated_example.snap"

1. The [`#[pavex::get]`][pavex::get] attribute is used to define a new route for the landing page.

We will refer to an item with a Pavex attribute as an **annotated item**.

The API reference has an _exhaustive_ list of all required and optional arguments for each attribute[^exhaustive].
Nonetheless, a few mechanisms are common to all attributes:

- [Generation (and customization) of component ids](component_id.md)
- [Syntax to annotate functions and methods](functions_and_methods.md)
- [Syntax to annotate types](types.md)

This guide focuses on these cross-cutting concerns.

[^exhaustive]: Check out [the reference for `#[pavex::get]`][pavex::get] as an example of the documentation format.
    [pavex::get]: /api_reference/pavex/attr.get.html#arguments
