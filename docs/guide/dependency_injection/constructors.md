# Constructors

To make a type injectable, you can **register a constructor** for it.  
Pavex will then invoke your constructor to create instances of that type when needed.

## Requirements

A constructor must satisfy a few requirements:

<div class="annotate" markdown>

- It must be a function, a method or a trait method.
- It must be public (1), importable from outside the crate it is defined in.
- It must return, as output, the type you want to make injectable.  
  [Constructors can be fallible](#constructors-can-fail): a constructor for a type `T` can return `Result<T, E>`,
  where `E` is an error type.

</div>

1. Constructors must be invoked in the generated code.
   The generated code lives in a separate crate, the [server SDK crate], hence the requirement.

Going back to our `User` example, this would be a valid signature for a constructor:

--8<-- "doc_examples/guide/dependency_injection/user_middleware/project-constructor_def.snap"

!!! warning

    Constructors can be either sync or async.  
    Check out 
    [the "Sync or async" section](../routing/request_handlers.md#sync-or-async) in the guide on request handlers
    to learn when to use one or the other.

## Registration

Once you have defined a constructor, you need to register it with the application [`Blueprint`][Blueprint]:

--8<-- "doc_examples/guide/dependency_injection/user_middleware/project-constructor_registration.snap"

[`Blueprint::constructor`][Blueprint::constructor] takes two arguments:

- An [unambiguous path](cookbook.md) to the constructor, wrapped in the [`f!`][f] macro.
- The [constructor's lifecycle](#lifecycles).

Alternatively, you could use [`Blueprint::request_scoped`][Blueprint::request_scoped] as 
a shorthand to perform the same registration:

--8<-- "doc_examples/guide/dependency_injection/user_middleware/02-constructor_registration.snap"

There is a shorthand for each lifecycle: [`Blueprint::singleton`][Blueprint::singleton], 
[`Blueprint::request_scoped`][Blueprint::request_scoped], [`Blueprint::transient`][Blueprint::transient].

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
[it will report an error](../../getting_started/quickstart/dependency_injection.md#missing-constructor).

## Constructors can fail

Constructors can be fallible: they can return a `Result<T, E>`, where `E` is an error type.  
If a constructor is fallible, you must specify an [**error handler**](../errors/error_handlers.md) when registering 
it with the application [`Blueprint`][Blueprint]. 
Check out the [error handling guide](../errors/error_handlers.md) for more details.

## Invocation order

Pavex provides no guarantees on the _relative_ invocation order of constructors.

Consider the following request handler:

--8<-- "doc_examples/guide/dependency_injection/core_concepts/project-handler.snap"

It injects two different types as input parameters, `A` and `B`.  
The way input parameters are ordered in `handler`'s definition does not influence the invocation order
of the respective constructors. Pavex may invoke `A`'s constructor before `B`'s constructor,
or vice versa.

The final invocation order will be primarily determined based on:

- **Dependency constraints**.  
  If `A`'s constructor takes `C` as input and `C`'s constructor takes `&B` as input,
  `B`'s constructor will certainly be invoked before `A`'s. There's no other way!
- **Borrow-checking constraints**.  
  If `A`'s constructor takes a reference to `C` as input, while `B`'s constructor takes `C` by value,
  Pavex will invoke `A`'s constructor first to avoid a `.clone()`.

## No mutations

Constructors are not allowed to take mutable references (i.e. `&mut T`) as inputs.  
It'd be quite difficult to reason about mutations since you can't control the
[invocation order of constructors](#invocation-order).

On the other hand, invocation order is well-defined for other types of components:
[request handlers](../routing/request_handlers.md),
[pre-processing middlewares](../middleware/pre_processing.md) and
[post-processing middlewares](../middleware/post_processing.md).
That's why Pavex allows them to inject mutable references as input parameters.

!!! note "Wrapping middlewares"

    Invocation order is well-defined for wrapping middlewares, but Pavex
    doesn't let them manipulate mutable references.  
    Check [their guide](../middleware/wrapping.md#use-with-caution) 
    to learn more about the rationale for this exception.


[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::constructor]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.constructor
[Blueprint::singleton]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.singleton
[Blueprint::request_scoped]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.request_scoped
[Blueprint::transient]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.transient
[f]: ../../api_reference/pavex/macro.f.html
[Lifecycle::Singleton]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton
[Lifecycle::RequestScoped]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.RequestScoped
[Lifecycle::Transient]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Transient
[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
