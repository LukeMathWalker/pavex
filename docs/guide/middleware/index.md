# Middleware

Middlewares execute logic before and/or after the route handler.
They are often used to implement **cross-cutting functionality**, such as:

- Telemetry (e.g. structured logging, metrics)
- Load-shedding (e.g. timeouts, rate-limiting)
- Access control (e.g. authentication, authorization)

## Middleware types

Pavex provides three types of middlewares: [pre-processing], [post-processing], and [wrapping middlewares].\
As the naming suggests, they differ in **when** they start and complete their execution, making them suitable for different use cases.

!!! note "Request processing pipeline"

    In this guide we'll often talk about the **request processing pipeline**.  
    This term refers to the sequence of components that handle a request, from the moment it arrives to the moment 
    the response is sent back to the caller.  
    It includes, in particular, the route handler and all the middlewares that apply to that route.

At a glance:

| Type              | Starts                   | Completes                | Suitable for                                                                                                                                                                     |
| ----------------- | ------------------------ | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Pre-processing]  | Before the route handler | Before the route handler | Skipping the remaining processing, returning an early response.<br/> Example: rejecting unauthenticated requests.                                                                |
| [Post-processing] | After the route handler  | After the route handler  | Modifying the response and/or performing side-effects based on the its contents.<br/>Examples: logging the response's status code, injecting headers.                            |
| [Wrapping]        | Before the route handler | After the route handler  | Accessing the future representing the rest of the request processing pipeline.<br/>Examples: enforcing a timeout, attaching a `tracing` span to the request processing pipeline. |

Each middleware type has a dedicated section in this guide. Check them out for more details!

[pre-processing]: ./pre_processing.md
[post-processing]: ./post_processing.md
[wrapping middlewares]: ./wrapping.md
[Pre-processing]: ./pre_processing.md
[Post-processing]: ./post_processing.md
[Wrapping]: ./wrapping.md
