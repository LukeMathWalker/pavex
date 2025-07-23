# Dependency injection

When working on a Pavex application, you don't have to worry about **wiring**.\
All the components in your application (request handlers, middlewares, error handlers, etc.) use their input parameters to declare the data they need to do their job.
We refer to those input parameters as their **dependencies**.
Pavex takes care of **building** and **injecting** those dependencies when and where they're needed.

We refer to this system as Pavex's **dependency injection framework**.

## What is the purpose of dependency injection?

Let's look at an example: rejecting unauthenticated requests in [a middleware](../middleware/index.md).

The desired behavior:

- If the user is logged in, the middleware lets the request through.
- If the user isn't logged in, a `401 Unauthorized` response is returned.

--8<-- "docs/examples/dependency_injection/user_middleware/authentication.snap"

The middleware logic doesn't care about _how_ authentication is performed. It only cares about
the result: is the user authenticated or not?

**The contract is data-driven**: as long as the outcome of the authentication process doesn't change
(i.e. the `User` type) the middleware will work as expected and doesn't need to be modified.\
You won't have to touch middleware code if, in the future,
you decide to migrate to a different authentication system
(e.g. from username/password authentication to an OAuth2 flow).

This is the entire purpose of Pavex's dependency injection framework: **decouple the way data is computed
from the way it's used**.
The middleware doesn't care about _how_ the `User` is computed, it only cares about _what_ it is.

This is a simple example, but the same principle applies to a vast collection of use cases:
body parsing, logging, authorization, etc.

## Guide structure

There are four different sources of injectable dependencies in Pavex:

- [**Framework primitives**](framework_primitives.md)
- [**Constructors**](constructors.md)
- [**Configuration**](../configuration/index.md)
- [**Prebuilt types**](prebuilt_types.md)

Check out the respective sections for guidance on how and when to use each.

## First-party components

Pavex provides a variety of constructors to extract commonly used data from the incoming request.
Check out the ["Request data"](../request_data/index.md) guide for an overview.
