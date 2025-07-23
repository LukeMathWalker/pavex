# URL encoded

[URL encoding](https://en.wikipedia.org/wiki/Percent-encoding), also known as _percent encoding_, is one
of the formats used by browsers to encode the data submitted via a `POST` web form.

You can use [`UrlEncodedBody<T>`][UrlEncodedBody] to work with URL encoded payloads:
it parses the raw request body into an instance of the type `T` you specified.

--8<-- "docs/examples/request_data/urlencoded/extraction.snap"

1. The parsed body is injected as an input parameter.

The whole request body is buffered in memory before being deserialized.

## Imports

To use [`UrlEncodedBody<T>`][UrlEncodedBody] in your application, you need to import its constructor from `pavex`:

--8<-- "docs/examples/request_data/urlencoded/registration.snap"

## Deserialization

The newly defined struct must be **deserializable**—i.e. it must implement
the [`serde::Deserialize`][serde::Deserialize] trait.\
You can derive [`serde::Deserialize`][serde::Deserialize] in most cases.

--8<-- "docs/examples/request_data/urlencoded/struct_def.snap"

## Unsupported field types

[`UrlEncodedBody<T>`][UrlEncodedBody] doesn't support deserializing nested structures.
For example, the following can't be deserialized from the wire using [`UrlEncodedBody<T>`][UrlEncodedBody]:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UpdateUserBody {
    address: Address
}

#[derive(Deserialize)]
pub struct Address {
    street: String,
    city: String,
}
```

If you need to deserialize nested structures from a URL encoded body,
you might want to look into writing your own extractor on top of a crate like
[`serde_qs`](https://crates.io/crates/serde_qs).

## Avoiding allocations

If you want to minimize memory usage, you can try to avoid unnecessary heap memory allocations when deserializing
string-like fields from the body of the incoming request.
Pavex supports this use case—**you can borrow from the request body**.

### Percent-encoding

It is not always possible to avoid allocations when handling a URL encoded body.\
Fields and values in a URL encoded body must comply with the restriction of the URI specification:
you can only use [a limited set of characters](https://datatracker.ietf.org/doc/html/rfc3986#section-2).\
If you want to use a character that's not URL-safe, you
must [percent-encode it](https://developer.mozilla.org/en-US/docs/Glossary/Percent-encoding).\
For example, if you want to use a space in a field name or a field value, you must encode it as `%20`.
A string like `John Doe` becomes `John%20Doe` when percent-encoded.

[`UrlEncodedBody<T>`][UrlEncodedBody] automatically decodes percent-encoded strings for you. But that comes at a cost:
Pavex _must_ allocate a new `String` if the value is percent-encoded.

### Cow

We recommend using [`Cow<'_, str>`][Cow] as your field type for string-like parameters.
It borrows from the buffered request body if possible, it allocates a new `String` if it can't be avoided.

[`Cow<'_, str>`][Cow] strikes a balance between performance and robustness: you don't have to worry about a runtime
error if the field is percent-encoded, but you minimise memory usage when it is.

[BufferedBody]: /api_reference/pavex/request/body/struct.BufferedBody.html
[UrlEncodedBody]: /api_reference/pavex/request/body/struct.UrlEncodedBody.html
[UrlEncodedBody::register]: /api_reference/pavex/request/body/struct.UrlEncodedBody.html#method.register
[serde::Deserialize]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
[Cow]: https://doc.rust-lang.org/std/borrow/enum.Cow.html
