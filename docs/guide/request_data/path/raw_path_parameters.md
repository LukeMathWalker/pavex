# Raw path parameters

[`PathParams<T>`][PathParams] is a high-level interface: it bundles together compile-time checks,
extraction and parsing.\
If you want to opt out of all those utilities, reach for [`RawPathParams`][RawPathParams].\
[`RawPathParams`][RawPathParams] is a lower-level interface[^relationship]: it gives you access to the dynamic
path segments as they appear right after extraction.
It doesn't perform percent-decoding nor deserialization.

## Injection

[`RawPathParams`][RawPathParams] is a [framework primitive](../../dependency_injection/framework_primitives.md),
you don't have to register a constructor to inject it.

--8<-- "doc_examples/guide/request_data/route_params/project-raw_route_params.snap"

## What does "raw" mean?

Path parameters must comply with the restriction of the URI specification:
you can only use [a limited set of characters](https://datatracker.ietf.org/doc/html/rfc3986#section-2).\
If you want to use a character not allowed in a URI, you must [percent-encode it](https://developer.mozilla.org/en-US/docs/Glossary/Percent-encoding).\
For example, if you want to send `123 456` as a route parameter, you must encode it as
`123%20456` where `%20` is a percent-encoded whitespace.

[`RawPathParams`][RawPathParams] gives you access to the **raw** route parameters, i.e. the route parameters
as they are extracted from the URL, before any kind of processing has taken
place.

In particular, [`RawPathParams`][RawPathParams] does **not** perform any percent-decoding.\
If you send a request to `/address/123%20456/home/789`, the [`RawPathParams`][RawPathParams] for
`/address/:address_id/home/:home_id` will contain the following key-value pairs:

- `address_id`: `123%20456`
- `home_id`: `789`

`address_id` is not `123 456` because [`RawPathParams`][RawPathParams] does not perform percent-decoding!
Therefore `%20` is not interpreted as a space character.

There are situations where you might want to work with the raw route parameters, but
most of the time you'll want to use [`PathParams`][PathParams] instead—it performs percent-decoding
and deserialization for you.

## Allocations

[`RawPathParams`][RawPathParams] tries to avoid heap memory allocations.\
Parameter names are borrowed from the server routing machinery.\
Parameter values are borrowed from the [raw path](index.md) of the incoming request.

You might have to allocate when [you perform percent-decoding][EncodedParamValue::decode].

[^relationship]: [`PathParams<T>`][PathParams] is built on top of [`RawPathParams`][RawPathParams].

[PathParams]: ../../../api_reference/pavex/request/path/struct.PathParams.html
[RawPathParams]: ../../../api_reference/pavex/request/path/struct.RawPathParams.html
[EncodedParamValue::decode]: ../../../api_reference/pavex/request/path/struct.EncodedParamValue.html#method.decode
