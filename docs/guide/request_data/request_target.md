# Request target

All incoming HTTP requests include a [target](https://datatracker.ietf.org/doc/html/rfc7230#section-5.3)
in the [request head](wire_data.md#requesthead).  
The target is a URI[^rfc], either in absolute form (e.g. `https://example.com/foo/bar?baz=qux`) or in
origin form (e.g. `/foo/bar?baz=qux`).

## URI components

A URI can be broken down into different components.
Let's take `https://example.com/foo/bar?baz=qux` as an example:

- The **scheme** is `https`.
- The **authority** is `example.com`.
- The **path** is `/foo/bar`.
- The **query** is `baz=qux`.

If the request target is in origin form, the authority and the scheme are omitted: you're left with just the path and the query,
e.g. `/foo/bar?baz=qux`.

## Injection

You can access the request target, as is, by injecting [`RequestHead`][RequestHead] and accessing its [`uri`][RequestHead::uri] field:

--8<-- "doc_examples/guide/request_data/request_target/project-target.snap"

## Use cases

The raw target and its components are primarily useful for logging purposes.  
We recommend
using our higher-level abstractions
to perform more advanced processing—e.g. parsing query parameters or route parameters.

[^rfc]: [RFC 7230](https://datatracker.ietf.org/doc/html/rfc7230#section-5.3) allows two other formats of request target,
authority form (e.g. `example.com:443`) and asterisk form (e.g. `*`).  
For both alternative formats there is a canonical conversion into a URI (_effective request target_). 
Pavex takes care of the conversion automatically; you can access [`RequestHead::uri`][RequestHead::uri] 
without having to worry about it.

[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
[RequestHead::uri]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.uri
