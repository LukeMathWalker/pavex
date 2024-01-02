# Wire representation

Pavex uses two types to model the request data as it arrives on the wire: [`RequestHead`][RequestHead]
and [`RawIncomingBody`][RawIncomingBody].  

## `RequestHead`

[`RequestHead`][RequestHead] encapsulates the data transmitted at the beginning of an HTTP request: [method][RequestHead::method],
[HTTP version][RequestHead::version], [request target](request_target.md)
and [headers][RequestHead::headers].  
All the data in the [`RequestHead`][RequestHead] has been read from the wire by the time Pavex
invokes your code.

### Injection

[`RequestHead`][RequestHead] is a [framework primitive](../dependency_injection/core_concepts/framework_primitives.md), 
you don't have to register a constructor to inject it.  

--8<-- "doc_examples/guide/request_data/wire_data/project-head.snap"

[`RequestHead`][RequestHead] is a dependency for a wide range of extractors.  
We recommend injecting a shared reference as input (i.e. `&RequestHead`)
rather than consuming [`RequestHead`][RequestHead] by value.

## `RawIncomingBody`

[`RawIncomingBody`][RawIncomingBody] gives you access to the raw body of the incoming HTTP request.  

It sits at the **lowest level of abstraction** when it comes to body processing.
You're looking at the stream of bytes coming from the network.
There are **no safeguards nor conveniences**.

In most situations, you're better off avoiding [`RawIncomingBody`][RawIncomingBody] entirely: prefer working with [the
higher-level body abstractions](body/index.md) provided by Pavex.

### Injection

[`RawIncomingBody`][RawIncomingBody] is a [framework primitive](../dependency_injection/core_concepts/framework_primitives.md),
you don't have to register a constructor to inject it.

--8<-- "doc_examples/guide/request_data/wire_data/project-body.snap"

Most abstractions built on top of [`RawIncomingBody`][RawIncomingBody] consume it by value.  
You can't really share an instance of [`RawIncomingBody`][RawIncomingBody]: you need exclusive access to pull
data from the stream of bytes.
Pavex will return a borrow-checking error if you try to consume the same [`RawIncomingBody`][RawIncomingBody] from different components.

## No `Request` type

There is no over-arching `Request` type in Pavex.  
The access patterns for the data in the request head (headers, method, path) 
and the body are different. The request head is primarily accessed through a shared reference
(i.e. `&RequestHead`) while the body is consumed by value (i.e. `RawIncomingBody`).  
By keeping them separate, we reduce the occurrence of annoying borrow-checking errors
in your day-to-day Pavex work.  

You can always inject both types if you need to look at the entire request at once.

[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
[RequestHead::version]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.version
[RequestHead::method]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.method
[RequestHead::headers]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.headers
[RequestHead::target]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.target
[RawIncomingBody]: ../../api_reference/pavex/request/body/struct.RawIncomingBody.html
