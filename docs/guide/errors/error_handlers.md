# Error handlers

Error handlers turn errors into HTTP responses.  
They are a mechanism to **decouple what went wrong from the way it's communicated to the caller**.

## Registration

You must specify an error handler every time you register a fallible component
(request handler, constructor, middleware).  

--8<-- "doc_examples/guide/errors/error_handlers/project-registration.snap"

1. Pavex will return an error during code generation if you register an error handler for an infallible component.

When registering an error handler, you must provide its **fully qualified path**, wrapped in the 
[`f!`][f] macro.  

!!! note "Registration syntax"

    You can use free functions, static methods, non-static methods, and trait methods as error handlers.
    Check out the [dependency injection cookbook](../dependency_injection/cookbook.md) for more details on
    the syntax for each case.

## `IntoResponse`

Error handlers, like request handlers and middlewares, must return a type that can be converted into a 
[`Response`][Response] via the [`IntoResponse`][IntoResponse] trait.  

--8<-- "doc_examples/guide/errors/error_handlers/project-into_response.snap"

1. Pavex implements `IntoResponse` for `StatusCode`, thus it's an acceptable return type for an error handler.

If you want to return a custom type from your error handler, you must implement [`IntoResponse`][IntoResponse] for it.

## Error reference

Error handlers must take a reference to the error type they're handling as one of their input parameters.  

--8<-- "doc_examples/guide/errors/error_handlers/project-into_response.snap"

1. In this example, `LoginError` is the error type returned by the fallible request handler.

## Error handlers can't fail

**Error handlers must be infallible**—i.e. they can't return a `Result`.  
Error handlers perform a **conversion**. The error type should contain all the information required to build the HTTP response. 
If that's not the case,
you may be tempted to perform fallible operations in the error handler to enrich the error type.
Resist the temptation!  
Instead, rework the fallible component to add the missing details to the error type, 
so that the error handler can be infallible.

## Dependency injection

Error handlers can take advantage of **dependency injection**.

You must specify the dependencies of your error handler as **input parameters** in its function signature.  
Those inputs are going to be built and injected by the framework, according to the **constructors** you have registered.

Check out the [dependency injection guide](../dependency_injection/index.md) for more details on how the process works.  

## Sync or async?

Error handlers are commonly synchronous, but Pavex supports asynchronous error handlers as well.  
Check out the ["Sync vs async"](../routing/request_handlers.md#sync-or-async) guide for more details
on the differences between the two and how to choose the right one for your use case.
  
[IntoResponse]: ../../../api_reference/pavex/response/trait.IntoResponse.html
[Response]: ../../../api_reference/pavex/response/struct.Response.html
[f]: ../../../api_reference/pavex/macro.f.html
