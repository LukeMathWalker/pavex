# On functions and methods

Most[^most] Pavex attributes must applied to a function or a method—something that can be _invoked_.

### Free functions

For free functions, it's as easy as it gets: the attribute goes on top of the function definition.

--8<-- "docs/examples/attributes/annotated_example.snap"

1. This function, annotated with [`#[pavex::get]`][pavex::get], will handle `GET /` requests.

### Methods

Methods require an extra bit of syntax: you must add [`#[pavex::methods]`][pavex::methods] to the `impl` block where the annotated method is defined.

--8<-- "docs/examples/attributes/method_annotation.snap"

1. The [`#[pavex::methods]`][pavex::methods] attribute is right on top of the `impl` block.

Pavex will raise an error at compile-time if you forget [`#[pavex::methods]`][pavex::methods].

### Trait methods

You can also use trait methods as Pavex components:

--8<-- "docs/examples/attributes/trait_method.snap"

1. The [`#[pavex::methods]`][pavex::methods] attribute is on top of the `impl` block, identical to the previous example.

[^most]: All attributes with the exception of [`#[pavex::config]`][pavex::config] and [`#[pavex::prebuilt]`][pavex::prebuilt], which are covered in [the next section on type annotations](types.md).

[pavex::methods]: /api_reference/pavex/attr.methods.html
[pavex::config]: /api_reference/pavex/attr.config.html
[pavex::prebuilt]: /api_reference/pavex/attr.prebuilt.html
[pavex::get]: /api_reference/pavex/attr.get.html
