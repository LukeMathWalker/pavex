# Limitations

Pavex's dependency injection system is powerful, but it's not perfect.\
Due to the [technology we're using under the hood](https://youtu.be/OxQYyg_v3rw?feature=shared),
there are some limitations you need to be aware of.

## Trait bounds are ignored

Pavex ignores trait bounds on generic parameters.

```rust
/// A generic parameter with a trait bound.
pub fn with_bound<T>(input: T) -> Output<T> 
    where T: Debug
{
    // [...]
}

/// A generic parameter without a trait bound.
pub fn without_bound<T>(input: T) -> Output<T> {
    // [...]
}
```

From Pavex's perspective, `with_bound` and `without_bound` are equivalent: they take `T` as input parameter and return `Output<T>`.

As a consequence, Pavex won't detect any errors related to trait bounds in the code-generation phase.
Those errors will be picked up by the Rust compiler when it tries to compile the generated code.

## Naked generics

A generic parameter is **naked** if it appears, as is, in the function signature of a constructor.

```rust
/// A naked generic output parameter.
pub fn naked_output<T>(/* ... */) -> T {
    // [...]
}
```

From Pavex's perspective, `naked_output` is a universal constructor: it can build any type.
It will therefore reject the constructor with an error message at compile time.

You can have a naked generic input parameter,
but only if it's also an [output-driven generic parameter](cookbook.md#output-driven-generics).\
There is no ambiguity in that case:
Pavex determines the concrete type of the input parameter from the output type of the constructor.

```rust
/// A naked output-driven parameter.
pub fn wrapper<T>(t: T) -> Custom<T> {
    // [...]
}
```

You can't have a naked generic input parameter that is input-driven.

```rust
/// A naked input-driven parameter.
pub fn naked_input<T: DatabaseService>(t: T) -> QueryResult {
    // [...]
}
```

Pavex [can't reason about trait bounds](#trait-bounds-are-ignored), therefore it isn't smart enough
to determine the concrete type of the input parameter based on the bounds you've specified.
