# Overview

The **path** is a component of the [request target](../request_target.md).\
E.g. `/foo/bar` is the path component in `https://example.com/foo/bar?baz=qux` or `/foo/bar?baz=qux`.

The path is primarily used for [routing requests to the right handlers](../../routing/index.md).\
The path can also be used to encode dynamic data—check out ["Path parameters"](path_parameters.md) for
more details.

## Injection

Inject [`RequestHead`][RequestHead] to access the raw path via its [`target`][RequestHead::target] field:

--8<-- "docs/examples/request_data/wire_data/raw_path.snap"

[RequestHead]: /api_reference/pavex/request/struct.RequestHead.html
[RequestHead::target]: /api_reference/pavex/request/struct.RequestHead.html#structfield.target
