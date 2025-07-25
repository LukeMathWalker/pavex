# Overview

The **query** is a component of the [request target](../request_target.md).\
E.g. `baz=qux` is the query component in `https://example.com/foo/bar?baz=qux` or `/foo/bar?baz=qux`.

The query is primarily used to **encode data** in `GET` requests and redirects.
Check out ["Query parameters"](query_parameters.md) for more details on how to extract structured data
out of the raw query.

## Injection

Inject [`RequestHead`][RequestHead] to access the raw query via its [`target`][RequestHead::target] field:

--8<-- "docs/examples/request_data/wire_data/raw_query.snap"

1. The query string is an optional component of the request target. It may not be there!

[RequestHead]: /api_reference/pavex/request/struct.RequestHead.html
[RequestHead::target]: /api_reference/pavex/request/struct.RequestHead.html#structfield.target
