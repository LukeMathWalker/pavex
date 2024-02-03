# Error observers

Error observers are a mechanism to **intercept errors**.  
They are primarily **designed for error reporting**—e.g. you can use them to log errors,
increment a metric counter, etc.


An error observer must:

- take a reference to [`pavex::Error`][pavex::Error] as one of its input parameters.
- have no return type (i.e. it must return `()`).
- be [strictly infallible](#infallible).

Error observers, like other components, can:

- Be [synchronous or asynchronous](../routing/request_handlers.md#sync-or-async).
- Take advantage of [dependency injection](../dependency_injection/index.md).

Error observers are invoked after the relevant error handler has been called,
but before the response is sent back to the client.  
You can register as many error observers as you want: they'll all be called when an error occurs,
in the order they were registered.

## Strictly infallible

Just like error handlers, error observers can't be fallible—they can't return a `Result`.  
It goes further than that, though: they **can't depend on fallible components**, neither directly nor indirectly.  
This constraint is necessary to **avoid infinite loops**.

Consider this scenario: you register an error observer that depends on a type `A`, and `A`'s constructor can fail.  
Something fails in the request processing pipeline:

- You want to invoke the error observer: you need to build `A`.
    - You invoke `A`'s constructor, but it fails!
        - You must now invoke the error observer on the error returned by `A`'s constructor.
            - But to invoke the error observer, you need `A`!
                - You try to construct `A` _again_ to report the failure of constructing `A`...

It never ends!


[pavex::Error]: ../../api_reference/pavex/struct.Error.html
