# Error observers

Error observers are a mechanism to **intercept errors**.\
They are primarily **designed for error reporting**—e.g. you can use them to log errors,
increment a metric counter, etc.

--8<-- "doc_examples/guide/errors/error_observers/project-example.snap"

## Registration

You register an error observer using the [`Blueprint::error_observer`][Blueprint::error_observer] method.

--8<-- "doc_examples/guide/errors/error_observers/project-registration.snap"

You must provide an **[unambiguous path]** to the error observer, wrapped in the [`f!`][f] macro.\
You can register as many error observers as you want: they'll all be called when an error occurs,
in the order they were registered. They are invoked after the relevant error handler has been called,
but before the response is sent back to the client.

!!! note "Registration syntax"

    You can use free functions, static methods, non-static methods, and trait methods as error handlers.
    Check out the [dependency injection cookbook](../dependency_injection/cookbook.md) for more details on
    the syntax for each case.

## `pavex::Error`

Error observers must take a reference to [`pavex::Error`][pavex::Error] as one of their input parameters.

--8<-- "doc_examples/guide/errors/error_observers/project-observer.snap"

[`pavex::Error`][pavex::Error] is an opaque error type—it's a wrapper around the actual error type returned by the
component that failed.\
It implements the [`Error`][std::error::Error] trait from the standard library, so you can use its methods
to extract information about the error (e.g. [`source`][std::error::Error::source], [`Display`][std::fmt::Display]
and [`Debug`][std::fmt::Debug] representations, etc.).\
If you need to access the underlying error type, you can use the [`inner_ref`][pavex::Error::inner_ref] method
and then try to [downcast it][std::error::Error::downcast_ref].

## Return type

The primary purpose of error observers is to **perform side effects**, not to produce a value.\
Therefore, they are expected to return the unit type, `()`—i.e. they don't return anything.

## Dependency injection

Error observers can take advantage of **dependency injection**.

--8<-- "doc_examples/guide/errors/error_observers/project-injection.snap"

1. `&RootSpan` is injected into the error observer by the framework.

You must specify the dependencies of your error observer as **input parameters** in its function signature.\
Those inputs are going to be built and injected by the framework, according to the **constructors** you have registered.

Check out the [dependency injection guide](../dependency_injection/index.md) for more details on how the process works.

## Strictly infallible

Just like error handlers, error observers can't be fallible—they can't return a `Result`.\
It goes further than that, though: they **can't depend on fallible components**, neither directly nor indirectly.\
This constraint is necessary to **avoid infinite loops**.

Consider this scenario: you register an error observer that depends on a type `A`, and `A`'s constructor can fail.\
Something fails in the request processing pipeline:

- You want to invoke the error observer: you need to build `A`.
  - You invoke `A`'s constructor, but it fails!
    - You must now invoke the error observer on the error returned by `A`'s constructor.
      - But to invoke the error observer, you need `A`!
        - You try to construct `A` _again_ to report the failure of constructing `A`...

It never ends!\
Pavex will detect this scenario and return an error during code generation, so that you don't end up
in an infinite loop at runtime.

## Sync or async?

Error observers can be either synchronous or asynchronous.\
Check out the ["Sync vs async"](../routing/request_handlers.md#sync-or-async) guide for more details
on the differences between the two and how to choose the right one for your use case.

[unambiguous path]: /guide/dependency_injection/cookbook.md#unambiguous-paths
[pavex::Error]: /api_reference/pavex/struct.Error.html
[pavex::Error::inner_ref]: /api_reference/pavex/struct.Error.html#method.inner_ref
[Blueprint::error_observer]: /api_reference/pavex/blueprint/struct.Blueprint.html#method.error_observer
[f]: /api_reference/pavex/macro.f.html
[std::error::Error]: https://doc.rust-lang.org/std/error/trait.Error.html
[std::error::Error::source]: https://doc.rust-lang.org/std/error/trait.Error.html#method.source
[std::fmt::Display]: https://doc.rust-lang.org/std/fmt/trait.Display.html
[std::fmt::Debug]: https://doc.rust-lang.org/std/fmt/trait.Debug.html
[std::error::Error::downcast_ref]: https://doc.rust-lang.org/std/error/trait.Error.html#method.downcast_ref
