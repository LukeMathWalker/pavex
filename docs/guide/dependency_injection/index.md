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

## Guide structure

"[Core concepts](core_concepts/constructors.md)" explains the key mechanics of Pavex's dependency injection framework.
It's a must-read if you're going to use Pavex for more than a toy project.

"[Cookbook](cookbook.md)" contains a collection of reference examples for common use cases: how to 
inject a function as a constructor, how to inject a non-static method, how to inject a trait object, generics, etc.  
Use it a reference in your day-to-day Pavex development if you're not sure of the syntax for a particular use case.

Pavex's dependency injection framework is a powerful tool, but it has its sharp edges.
"[Limitations](limitations.md)" covers the issues you're most likely to encounter and how to work around them.