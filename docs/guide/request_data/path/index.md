# Overview

The **path** is a component of the [request target](../request_target.md).  
E.g. `/foo/bar` is the path component in `https://example.com/foo/bar?baz=qux` or `/foo/bar?baz=qux`.

The path is primarily used for [routing requests to the right handlers](../../routing/index.md).  
The path can also be used to encode dynamic data—check out ["Route parameters"](route_parameters.md) for
more details.

## Injection

Inject [`RequestHead`][RequestHead] to access the raw path via its [`target`][RequestHead::target] field:

--8<-- "doc_examples/guide/request_data/path/project-raw_path.snap"

[RequestHead]: ../../../api_reference/pavex/request/struct.RequestHead.html
[RequestHead::target]: ../../../api_reference/pavex/request/struct.RequestHead.html#structfield.target