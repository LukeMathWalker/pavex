# Wire representation

Pavex uses two types to model the request data as it arrives on the wire: [`RequestHead`][RequestHead]
and [`RawIncomingBody`][RawIncomingBody].  

## `RequestHead`

[`RequestHead`][RequestHead] encapsulates the data transmitted at the beginning of an HTTP request: [method][RequestHead::method],
[HTTP version][RequestHead::version], [request target][RequestHead::uri] (modeled as a URI)
and [headers][RequestHead::headers].  
All the data in the [`RequestHead`][RequestHead] has been read from the wire by the time Pavex
invokes your code.

### Injection

[`RequestHead`][RequestHead] is a [framework primitive](../dependency_injection/core_concepts/framework_primitives.md), 
you don't have to register a constructor to inject it.  

[`RequestHead`][RequestHead] is a dependency for a wide range of extractors.  
We recommend injecting a shared reference as input (i.e. `&RequestHead`) in your handlers and middlewares 
rather than consuming [`RequestHead`][RequestHead] by value.


## No `Request` type

There is no over-arching `Request` type in Pavex.  
The access patterns for the data in the request head (headers, method, path) 
and the body are different. The request head is primarily accessed through a shared reference
(i.e. `&RequestHead`) while the body is taken by value (i.e. `RawIncomingBody`).  
By keeping them separate, we reduce the occurrence of annoying borrow-checking errors
in your day-to-day Pavex work.  

You can always inject both types if you need to look at the entire request at once.

[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
[RequestHead::version]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.version
[RequestHead::method]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.method
[RequestHead::headers]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.headers
[RequestHead::uri]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.uri
[RawIncomingBody]: ../../api_reference/pavex/request/body/struct.RawIncomingBody.html
