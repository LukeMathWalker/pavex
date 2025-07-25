# On types

[`#[pavex::config]`][pavex::config] and [`#[pavex::prebuilt]`][pavex::prebuilt] must be applied to types, rather than [functions and methods](functions_and_methods.md).

### Definitions

You can annotate struct, enum and type alias definitions:

--8<-- "docs/examples/attributes/struct.snap"

1. [`#[pavex::config]`][pavex::config] goes on top of the struct definition. It doesn't have to be above other attributes, unless they modify the struct definition.

### Re-exports

You may wish to use types defined in another crate as Pavex components.
If they've been annotated by the library author, it's just a matter of importing them into your blueprint. But what if they haven't?

You can't add annotations to types defined in third-party crates. However, you can still use them as Pavex components via an **annotated re-export**:

--8<-- "docs/examples/attributes/use.snap"

1. You can't apply [`#[pavex::prebuilt]`][pavex::prebuilt] where `reqwest::Client` is defined, since it's in a third-party crate. But you can avoid boilerplate by annotating its re-export.

This is equivalent to annotating the definition of `reqwest::Client` directly with [`#[pavex::prebuilt]`][pavex::prebuilt].

[pavex::config]: /api_reference/pavex/attr.config.html
[pavex::prebuilt]: /api_reference/pavex/attr.prebuilt.html
