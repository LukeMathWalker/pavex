# Generics

As a general rule, Pavex components aren't allowed to have generic type parameters.

There is one exception: the signature of a constructor can include generic type parameters, as long as they are [**output-driven**](#output-driven).

## Output-driven

A generic parameter is **output-driven** if it appears in the output type of a constructor.
If the constructor is fallible, the generic parameter must appear in type of the `Ok` variant to
qualify as output-driven.

Pavex will automatically infer the concrete type of output-driven parameters based on the signature of the component that's injecting
the constructed type.

## All generics must be output-driven

Pavex will reject, at compile-time, any component with generic parameters that are not output-driven.
