# Cookbook

This cookbook contains a collection of reference examples for Pavex's dependency injection framework.  
It covers the registration syntax for common use cases (free functions, methods) as well as more advanced ones
(trait methods, generics, etc.).

Use it a reference in your day-to-day Pavex development if you're not sure of the syntax for a particular use case.

!!! note "More than constructors"

    All the examples register constructors, but the very same `f!` invocations can be used to register 
    request handlers, error handlers, error observers and middlewares.

## Fully qualified paths

In all the cases described in this cookbook, we'll talk about **fully qualified paths**. 
They allow Pavex to unambiguously identify the callable you want to register.  
The format differs between [local items](#local-items) and [items imported from a dependency of your crate](#from-a-dependency).

### Local items

If the item you want to register is defined in the current crate,
you can use three different formats for its fully qualified path:

- A path prefixed with `crate`, spelling out where the item is located **with respect to the root of the crate**.
```rust
//! You can use `crate::my_module::handler` as a fully qualified path
//! for the `handler` function from anywhere in your crate.

pub mod my_module {
   pub fn handler() {
      // [...]
   }
}
```
- A path prefixed with `self`, spelling out where the item is located **with respect to the root of the current module**.
```rust
pub mod my_module {
   //! From inside `my_module`, you can use `self::handler` as a fully qualified path
   //! for the `handler` function.
   pub fn handler() {
      // [...]
   }
}
```
- A path prefixed with `super`, spelling out where the item is located **with respect to the parent of the current module**.
```rust
pub fn handler() {
   // [...]
}

pub mod my_module {
   //! From inside `my_module`, you can use `super::handler` as a fully qualified path
   //! for the `handler` function.
}
```

The three formats are equivalent for Pavex.
In practice,
paths prefixed with `self` or `super` often end up being shorter—prefer them if you want to make your registration code terser.

### From a dependency

If the item is defined in a dependency of your crate, you must use its absolute path, starting the name of 
the crate it is defined into. E.g. `reqwest::Client`, if `reqwest` is one of your dependencies.

## Free functions

Free functions are the simplest case.
You register them as constructors by passing their [fully qualified path] to the [`f!` macro][f!].

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-function_registration.snap"

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-function_def.snap"

## Static methods

Static methods (1) behave exactly like free functions:
you can register them as constructors by passing their [fully qualified path] to the [`f!` macro][f!].
{ .annotate }

1. A static method is a method that doesn't take `self` (or one of its variants) as an input parameter.

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-static_method_registration.snap"

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-static_method_def.snap"

## Non-static methods

On the surface, non-static methods are registered in the same way as static methods: 
by passing their [fully qualified path] to the [`f!` macro][f!].

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-non_static_method_registration.snap"

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-non_static_method_def.snap"

However, there's a catch: `self` counts as a dependency!  
When registering a non-static method, you need to make sure to also register a constructor
for the type of `self`.

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-non_static_method_self_constructor_registration.snap"

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-non_static_method_self_constructor_def.snap"

## Trait methods

The syntax for trait methods (1) is a bit more complex: you need to use the fully qualified syntax
for function calls[^ufcs].
{ .annotate }

1. An inherent method is defined directly on a type (e.g. `Vec::new`).
   A trait method is part of trait definition (e.g. `Iterator::next`) and it's available
   on a type if that type implements the trait.

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-trait_method_registration.snap"

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-trait_method_def.snap"


## Generics

### Output-driven generics

A generic parameter is **output-driven** if it appears in the output type of a constructor.  

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-output_def.snap"

If the constructor is fallible, the generic parameter must appear in type of the `Ok` variant to
qualify as output-driven.

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-fallible_output_def.snap"

If all generic parameters are output-driven, you can register the constructor
as if it wasn't generic. Pavex will automatically infer the generic parameters
based on the scenarios where the constructor is used.

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-output_registration.snap"

### Input-driven generics

A generic parameter is **input-driven** if it isn't output-driven, i.e. it doesn't appear in the output type of a 
constructor.  

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-input_def.snap"

If all generic parameters are input-driven, you need to explicitly specify
the concrete type for each generic parameter when registering the constructor.

--8<-- "doc_examples/guide/dependency_injection/cookbook/project-input_registration.snap"

### Mixed generics

If a constructor has both input-driven and output-driven generic parameters,
you need to explicitly specify the concrete type for all generic parameters
when registering the constructor.


[f!]: ../../api_reference/pavex/macro.f!.html
[fully qualified path]: #fully-qualified-paths
[^ufcs]: Check out the [relevant RFC](https://github.com/rust-lang/rfcs/blob/master/text/0132-ufcs.md) if you're curious.