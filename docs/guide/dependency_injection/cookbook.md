# Cookbook

This cookbook contains a collection of reference examples for Pavex's dependency injection framework.  
It covers common use cases (free functions, methods) as well as more advanced ones (trait methods, generics, etc.).

Use it a reference in your day-to-day Pavex development if you're not sure of the syntax for a particular use case.

## Free functions

Free functions are the simplest case.
You register them as constructors by passing their fully qualified path to the [`f!` macro][f!].

## Static methods

Static methods (1) behave exactly like free functions:
you can register them as constructors by passing their fully qualified path to the [`f!` macro][f!].
{ .annotate }

1. A static method is a method that doesn't take `self` (or one of its variants) as an input parameter.

## Non-static methods

On the surface, non-static methods are registered in the same way as static methods: 
by passing their fully qualified path to the [`f!` macro][f!].

However, there's a catch: `self` counts as a dependency!  
When registering a non-static method, you need to make sure to also register a constructor
for the type of `self`.

## Trait methods

The syntax for trait methods (1) is a bit more complex: you need to use the fully qualified syntax
for function calls[^ufcs].
{ .annotate }

1. An inherent method is defined directly on a type (e.g. `Vec::new`).
   A trait method is part of trait definition (e.g. `Iterator::next`) and it's available
   on a type if that type implements the trait.

## Generics

### Output-driven generics

A generic parameter is **output-driven** if it appears in the output type of a constructor.  

If the constructor is fallible, the generic parameter must appear in type of the `Ok` variant.

If all generic parameters are output-driven, you can register the constructor
as if it wasn't generic. Pavex will automatically infer the generic parameters
based on the scenarios where the constructor is used.

### Input-driven generics

A generic parameter is **input-driven** if it isn't output-driven, i.e. it doesn't appear in the output type of a 
constructor.  

If all generic parameters are input-driven, you need to explicitly specify
the concrete type for each generic parameter when registering the constructor.

### Mixed generics

If a constructor has both input-driven and output-driven generic parameters,
you need to explicitly specify the concrete type for each generic parameter
when registering the constructor, even if it's output-driven.


[f!]: ../../api_reference/pavex/macro.f!.html
[^ufcs]: Check out the [relevant RFC](https://github.com/rust-lang/rfcs/blob/master/text/0132-ufcs.md) if you're curious.