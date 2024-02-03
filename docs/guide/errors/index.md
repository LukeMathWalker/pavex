# Overview

[Request handlers](../routing/request_handlers.md#request-handlers-can-fail), 
[constructors](../dependency_injection/core_concepts/constructors.md), [middlewares](../middleware/index.md#middlewares-can-fail): 
they can all be **fallible**.  

--8<-- "doc_examples/guide/errors/error_handlers/project-fallible.snap"

What happens when they fail, though? What does the framework do with the error?  
Two different concerns must be addressed:

- **Reacting**: whoever called your API is waiting for a response! The error must be converted into an HTTP response.
- **Reporting**: you need to know when something goes wrong—and why.  
  You must be able to _report_ that an error occurred using your preferred monitoring system (e.g. 
  a log record, incrementing a counter, sending a notification, etc.).

These two concerns are addressed by two different Pavex components: [**error handlers**](error_handlers.md) 
and [**error observers**](error_observers.md).

!!! note

    Check out [this article](https://www.lpalmieri.com/posts/error-handling-rust/) for a deep dive 
    on the topic of error handling (in Rust and beyond).
