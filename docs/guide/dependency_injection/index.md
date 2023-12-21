# Overview

When working on a Pavex application, you don't have to worry about **wiring**.  
All the components in your application (request handlers, middlewares, error handlers, etc.) declare,
as input parameters, the data they need to do their job.
We refer to those input parameters as their **dependencies**.  
Pavex takes care of **constructing** those dependencies and **injecting** them where they're needed.

We refer to this system as Pavex's **dependency injection framework**.

## What is the purpose of dependency injection?

Let's look at an example: rejecting unauthenticated requests in a middleware.  

The desired behavior:

- If you're logged in, the middleware lets the request through.  
- If you're not, a `401 Unauthorized` response is returned.

--8<-- "doc_examples/guide/dependency_injection/user_middleware/project-middleware_def.snap"

The middleware logic doesn't care about _how_ authentication is performed. It only cares about
the result: are you authenticated or not?

**The contract is data-driven**: as long as the outcome of the authentication process doesn't change
(i.e. the `User` type) the middleware will work as expected and doesn't need to be modified.  
You won't have to touch middleware code if, in the future,
you decide to migrate to a different authentication system
(e.g. from username/password authentication to an OAuth2 flow).

This is the entire purpose of Pavex's dependency injection framework: **decouple the way data is computed
from the way it's used**.
The middleware doesn't care about _how_ the `User` is computed, it only cares about _what_ it is.

This is a simple example, but the same principle applies to a vast collection of use cases:
body parsing, logging, authorization, etc.

## Constructors

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
    [the "Sync or async" section](../routing/request_handlers.md#sync-or-async) in the guide on request handlers
    to learn when to use one or the other.

### Registration 

Once you have defined a constructor, you need to register it with the application [`Blueprint`][Blueprint]
using its [`constructor`][Blueprint::constructor] method:

--8<-- "doc_examples/guide/dependency_injection/user_middleware/project-constructor_registration.snap"

[`constructor`][Blueprint::constructor] takes two arguments:

- The fully qualified path to the constructor, wrapped in a macro ([`f!`][f])
- The **constructor's lifecycle**. 

### Lifecycles

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
| HTTP client              | [Singleton][Lifecycle::Singleton]         | Most HTTP clients keep, under the hood, an HTTP connection pool. <br/>You want to reuse those connections across requests to minimise latency and the number of open file descriptors.                                                                                                                                                                                                   |
| Route parameters         | [RequestScoped][Lifecycle::RequestScoped] | Route parameters are extracted from the incoming request. <br/> They must not be shared across requests, therefore they can't be a [`Singleton`][Lifecycle::Singleton].<br/> They could be [`Transient`][Lifecycle::Transient], but re-parsing the parameters before every use would be expensive.<br/>[`RequestScoped`][Lifecycle::RequestScoped] is the optimal choice.                |
| Database connection | [Transient][Lifecycle::Transient]         | The connection is retrieved from a shared pool.<br/>It could be [`RequestScoped`][Lifecycle::RequestScoped], but you might end up keeping the connection booked (i.e. outside of the pool) for longer than it's strictly necessary.<br/>[`Transient`][Lifecycle::Transient] is the optimal choice: you only remove the connection from the pool when it's needed, put it back when idle. |                                                                                                                                                                                                                                                                                                                          |

### Recursive dependencies

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

## Framework primitives

You don't have to register a constructor for every type you want to inject.  
Pavex provides a few types, called **framework primitives**, that you can inject
without having to register a constructor for them.

The framework primitives are:

- [`RequestHead`][RequestHead]. The incoming request data, minus the body.
- [`RawIncomingBody`][RawIncomingBody]. The raw body of the incoming request.
- [`RouteParams`][RouteParams]. The route parameters extracted from the incoming request.
- [`AllowedMethods`][AllowedMethods]. The HTTP methods allowed for the current request path.

They represent raw data from the incoming request ([`RequestHead`][RequestHead], [`IncomingBody`][IncomingBody]) 
or information coming from the routing system ([`AllowedMethods`][AllowedMethods], [`RouteParams`][RouteParams]).

### Convenient, but inflexible

As a [design philosophy](../../overview/why_pavex.md), Pavex strives to be **flexible**.
You should be allowed to customize the framework to your needs, without having to fight against it
or having to give up significant functionality.  
In particular, you should be able to change the way a certain type is constructed, even if that
type is defined in the `pavex` crate. For example, you might want to change the JSON deserializer used to parse the incoming request body
and produce a [`JsonBody<T>`][JsonBody] instance.  
You lose this flexibility with framework primitives: you can't customize how they are constructed. 
That's why we try to keep their number to a minimum.

## `ApplicationState`

All the [`Singleton`][Lifecycle::Singleton] types that your application needs to access at runtime
when processing a request are grouped together in a struct called [`ApplicationState`][ApplicationState].  
The [`ApplicationState`][ApplicationState] is code-generated by Pavex:
you don't have to write it yourself!  

It is defined in the [server SDK crate] and exported as a public type.  
It is then used in the [server crate] as an input parameter
to the [`run` function](../project_structure/index.md#run),
the one that launches the HTTP server to start listening for incoming requests.

### Singleton dependencies

Constructors for [`Singleton`][Lifecycle::Singleton] types are invoked before the application starts, 
in the generated [`build_application_state`][build_application_state] function.

[`ApplicationState`][ApplicationState] is designed to be minimal: it only includes the types that are needed **at runtime**.  
If a [`Singleton`][Lifecycle::Singleton] type is only needed to build another [`Singleton`][Lifecycle::Singleton] type,
it won't be included in the [`ApplicationState`][ApplicationState].
It will be constructed in [`build_application_state`][build_application_state] and then discarded.

If [`Singleton`][Lifecycle::Singleton] type `A` is needed to build [`Singleton`][Lifecycle::Singleton] type `B`,
you don't _have to_ register a constructor for `A`.  
Pavex will change the signature of [`build_application_state`][build_application_state] to require `A` as input parameter:
you're then free to build `A` however you want in the [server crate].


[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::constructor]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.constructor
[f]: ../../api_reference/pavex/macro.f.html
[Lifecycle::Singleton]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton
[Lifecycle::RequestScoped]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.RequestScoped
[Lifecycle::Transient]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Transient
[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
[RouteParams]: ../../api_reference/pavex/request/route/struct.RouteParams.html
[AllowedMethods]: ../../api_reference/pavex/router/enum.AllowedMethods.html
[RawIncomingBody]: ../../api_reference/pavex/request/body/struct.RawIncomingBody.html
[JsonBody]: ../../api_reference/pavex/request/body/struct.JsonBody.html
[build_application_state]: ../project_structure/index.md#applicationstate
[server SDK crate]: ../project_structure/index.md#the-server-sdk
[server crate]: ../project_structure/index.md#the-server-crate
[ApplicationState]: ../project_structure/index.md#applicationstate