# UrlEncoded

[`UrlEncodedBody<T>`][UrlEncodedBody] buffers the body in memory and deserializes it as URL-encoded,
according to the type `T` you specify.

## Registration

To use [`UrlEncodedBody<T>`][UrlEncodedBody] in your application you need to register a constructor for it.  
You can use [`UrlEncodedBody::register`][UrlEncodedBody::register] to register the default constructor
and error handler:

--8<-- "doc_examples/guide/request_data/urlencoded/project-installation.snap"

1. You also need to register a constructor for [`BufferedBody`][BufferedBody]!  
   Check out the [BufferedBody guide](../byte_wrappers.md) for more details.

If you're using the default [`ApiKit`](../../../dependency_injection/core_concepts/kits.md),
you don't need to register a constructor for [`BufferedBody`][BufferedBody] manually:
it's already included in the kit.

## Extraction

Inject [`UrlEncodedBody<T>`][UrlEncodedBody] as an input in your components to access the parsed body:

--8<-- "doc_examples/guide/request_data/urlencoded/project-extraction.snap"

## Deserialization

The newly defined struct must be **deserializable**—i.e. it must implement
the [`serde::Deserialize`][serde::Deserialize] trait.  
You can derive [`serde::Deserialize`][serde::Deserialize] in most cases.

--8<-- "doc_examples/guide/request_data/urlencoded/project-struct_with_attr.snap"

## Avoiding allocations

If you want to minimize memory usage, you can try to avoid unnecessary heap memory allocations when deserializing
string-like fields from the body of the incoming request.
Pavex supports this use case—**you can borrow from the request body**.

### Percent-encoding

It is not always possible to avoid allocations when handling an urlencoded body.  
A urlencoded body must comply with the restriction of the URI specification:
you can only use [a limited set of characters](https://datatracker.ietf.org/doc/html/rfc3986#section-2).  
If you want to use a character not allowed in a URI, you
must [percent-encode it](https://developer.mozilla.org/en-US/docs/Glossary/Percent-encoding).  
For example, if you want to use a space in a field name or a field value, you must encode it as `%20`.
A string like `John Doe` becomes `John%20Doe` when percent-encoded.

[`UrlEncodedBody<T>`][UrlEncodedBody] automatically decodes percent-encoded strings for you. But that comes at a cost:
Pavex _must_ allocate a new `String` if the route parameter is percent-encoded.

### Cow

We recommend using [`Cow<'_, str>`][Cow] as your field type for string-like parameters.
It borrows from the request's path if possible, it allocates a new `String` if it can't be avoided.

[`Cow<'_, str>`][Cow] strikes a balance between performance and robustness: you don't have to worry about a runtime
error if the route parameter
is percent-encoded, but you tried to use `&str` as its field type.

[BufferedBody]: ../../../../api_reference/pavex/request/body/struct.BufferedBody.html

[UrlEncodedBody]: ../../../../api_reference/pavex/request/body/struct.UrlEncodedBody.html

[UrlEncodedBody::register]: ../../../../api_reference/pavex/request/body/struct.UrlEncodedBody.html#method.register

[serde::Deserialize]: https://docs.rs/serde/latest/serde/trait.Deserialize.html

[Cow]: https://doc.rust-lang.org/std/borrow/enum.Cow.html
