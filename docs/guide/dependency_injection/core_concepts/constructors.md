# Constructors

To make a type injectable, you need to **register a constructor** for it.

A constructor must satisfy a few requirements:

<div class="annotate" markdown>

- It must be a function, a method or a trait method.
- It must be public (1), importable from outside the crate it is defined in.
- It must return, as output, the type you want to make injectable.  
  Constructors can be fallible: a constructor for a type `T` can return `Result<T, E>`,
  where `E` is an error type.

</div>

1. Constructors must be invoked in the generated code.
   The generated code lives in a separate crate, the [server SDK crate], hence the requirement.

Going back to our `User` example, this would be a valid signature for a constructor:

--8<-- "doc_examples/guide/dependency_injection/user_middleware/project-constructor_def.snap"

!!! warning

    Constructors can be either sync or async.  
    Check out 
    [the "Sync or async" section](../../routing/request_handlers.md#sync-or-async) in the guide on request handlers
    to learn when to use one or the other.

## Registration

Once you have defined a constructor, you need to register it with the application [`Blueprint`][Blueprint]
using its [`constructor`][Blueprint::constructor] method:

--8<-- "doc_examples/guide/dependency_injection/user_middleware/project-constructor_registration.snap"

[`constructor`][Blueprint::constructor] takes two arguments:

- The [fully qualified path](../cookbook.md) to the constructor, wrapped in a macro ([`f!`][f])
- The **constructor's lifecycle**.

## Lifecycles

Pavex supports three different lifecycles for constructors:

- [`Singleton`][Lifecycle::Singleton].
  The constructor is invoked **at most once**, before the application starts.  
  The same instance is injected every time the type is needed.
- [`RequestScoped`][Lifecycle::RequestScoped]. The constructor is invoked **at most once per request**.  
  The same instance is injected every time the type is needed when handling the same request.
- [`Transient`][Lifecycle::Transient]. The constructor is invoked **every time the type is needed**.  
  The injected instance is always newly created.

Let's look at a few common scenarios to build some intuition around lifecycles:

| Scenario                 | Lifecycle                                 | Why?                                                                                                                                                                                                                                                                                                                                                                                     |
|--------------------------|-------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Database connection pool | [Singleton][Lifecycle::Singleton]         | The entire application should use the same pool. <br/>Each request will fetch a connection from the pool when needed.                                                                                                                                                                                                                                                                    |
| HTTP client              | [Singleton][Lifecycle::Singleton]         | Most HTTP clients keep, under the hood, a connection pool. <br/>You want to reuse those connections across requests to minimise latency and the number of open file descriptors.                                                                                                                                                                                                   |
| Path parameters         | [RequestScoped][Lifecycle::RequestScoped] | Path parameters are extracted from the incoming request. <br/> They must not be shared across requests, therefore they can't be a [`Singleton`][Lifecycle::Singleton].<br/> They could be [`Transient`][Lifecycle::Transient], but re-parsing the parameters before every use would be expensive.<br/>[`RequestScoped`][Lifecycle::RequestScoped] is the optimal choice.                |
| Database connection | [Transient][Lifecycle::Transient]         | The connection is retrieved from a shared pool.<br/>It could be [`RequestScoped`][Lifecycle::RequestScoped], but you might end up keeping the connection booked (i.e. outside of the pool) for longer than it's strictly necessary.<br/>[`Transient`][Lifecycle::Transient] is the optimal choice: you only remove the connection from the pool when it's needed, put it back when idle. |                                                                                                                                                                                                                                                                                                                          |

## Recursive dependencies

Dependency injection wouldn't be very useful if all constructors were required to take no input parameters.  
The dependency injection framework is **recursive**: constructors can take advantage of dependency injection
to request the data they need to do their job.

Going back to our `User` example: it's unlikely that you'll be able to build a `User` instance without
taking a look at the incoming request, or some data extracted from it.

Let's say you want to build a `User` instance based on the value of the `Authorization` header
of the incoming request.  
You could define a constructor like this:

--8<-- "doc_examples/guide/dependency_injection/user_middleware/01-constructor_def.snap"

[`RequestHead`][RequestHead] represents the incoming request data, minus the body.  
When Pavex examines your application [`Blueprint`][Blueprint], the following happens:

- The `reject_anonymous` middleware must be invoked. Does `reject_anonymous` have any input parameters?
    - Yes, it needs a `User` instance. Do we have a constructor for `User`?
        - Yes, we do: `User::extract`. Does `User::extract` have any input parameters?
            - Yes, it needs a reference to a `RequestHead`. Do we have a constructor for `RequestHead`?
                - Etc.

The recursion continues until Pavex finds a constructor that doesn't have any input parameters or
a type that doesn't need to be constructed.  
If a type needs to be constructed, but Pavex can't find a constructor for it,
[it will report an error](../../../getting_started/quickstart/dependency_injection.md#missing-constructor).

## No mutations

Constructors are not allowed to take mutable references (i.e. `&mut T`) as inputs.

The order in which constructors are called would suddenly matter if they were allowed to mutate
their dependencies.
This would in turn require a way to specify that order, therefore increasing the overall complexity of the
framework.

[Blueprint]: ../../../api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::constructor]: ../../../api_reference/pavex/blueprint/struct.Blueprint.html#method.constructor
[f]: ../../../api_reference/pavex/macro.f.html
[Lifecycle::Singleton]: ../../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton
[Lifecycle::RequestScoped]: ../../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.RequestScoped
[Lifecycle::Transient]: ../../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Transient
[RequestHead]: ../../../api_reference/pavex/request/struct.RequestHead.html
