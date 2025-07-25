# Bytes

[`BufferedBody`][BufferedBody] consumes the [raw byte stream](../wire_data.md#rawincomingbody) and
buffers the entire body of the incoming request in memory.
At the same time, it takes care of enforcing [sane limits](#body-size-limit) to prevent resource exhaustion attacks.

## Imports

To use [`BufferedBody`][BufferedBody] in your application, you need to import its constructor from `pavex`:

--8<-- "docs/examples/request_data/buffered_body/registration.snap"

## Use cases

[`BufferedBody`][BufferedBody] is the ideal building block for other extractors that need to have the entire body
available in memory to do their job (e.g. [`JsonBody`][JsonBody]).

[`BufferedBody`][BufferedBody] is also a good fit if you need to access the raw bytes of the
body ahead of deserialization (e.g. to compute its hash as a step of a signature verification process).
In those scenarios, make sure to inject a shared reference to [`BufferedBody`][BufferedBody] (i.e. `&BufferedBody`)
into your component rather than consuming it (i.e. `BufferedBody`).

--8<-- "docs/examples/request_data/buffered_body/extraction.snap"

## Body size limit

[BufferedBody] enforces an upper limit on the body size to prevent [resource exhaustion attacks](https://owasp.org/API-Security/editions/2023/en/0xa4-unrestricted-resource-consumption/).
The default limit is 2 MBs.\
[BufferedBody::extract] returns [SizeLimitExceeded] if the limit is exceeded.

### Custom limit

You can customize the limit by registering a custom constructor for [BodySizeLimit] in your [Blueprint]:

--8<-- "docs/examples/request_data/buffered_body/custom_limit.snap"

### No limit

You can also disable the limit altogether:

--8<-- "docs/examples/request_data/buffered_body/no_limit.snap"

### Granular limits

In large applications with many routes it can be hard
(if not impossible) to find a single limit that works for all routes.
You can leverage nesting to define more granular limits.

--8<-- "docs/examples/request_data/buffered_body/granular_limits.snap"

[BufferedBody]: /api_reference/pavex/request/body/struct.BufferedBody.html
[BufferedBody::register]: /api_reference/pavex/request/body/struct.BufferedBody.html#method.register
[JsonBody]: /api_reference/pavex/request/body/struct.JsonBody.html
[BufferedBody::extract]: /api_reference/pavex/request/body/struct.BufferedBody.html#method.extract
[SizeLimitExceeded]: /api_reference/pavex/request/body/errors/enum.ExtractBufferedBodyError.html#variant.SizeLimitExceeded
[BodySizeLimit]: /api_reference/pavex/request/body/enum.BodySizeLimit.html
[Blueprint]: /api_reference/pavex/struct.Blueprint.html
