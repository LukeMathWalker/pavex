# Overview

The **query** is a component of the [request target](../request_target.md).  
E.g. `baz=quex` is the query component in `https://example.com/foo/bar?baz=qux` or `/foo/bar?baz=qux`.

The query is primarily used to **encode data** in `GET` requests and redirects.
Check out ["Query parameters"](query_parameters.md) for more details on how to extract structured data
out of the raw query.

## Injection

Inject [`RequestHead`][RequestHead] to access the raw query via its [`uri`][RequestHead::target] field:

--8<-- "doc_examples/guide/request_data/query/project-raw_query.snap"

1. The query string is an optional component of the request target. It may not be there!

[RequestHead]: ../../../api_reference/pavex/request/struct.RequestHead.html
[RequestHead::target]: ../../../api_reference/pavex/request/struct.RequestHead.html#structfield.target
