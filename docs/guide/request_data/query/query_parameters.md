# Query parameters

In REST APIs, the [query](index.md) is often used to encode data.  
For example, in `/search?sorted=true`,
the query is `sorted=true` and it's used to encode a `sorted` variable set to `true`.

Those variables are called **query parameters**. You can extract them using [`QueryParams<T>`][QueryParams].

## Registration

To use [`QueryParams<T>`][QueryParams] in your application you need to register a constructor for it.  
You can use [`QueryParams::register`][QueryParams::register] to register its default constructor
and error handler:

--8<-- "doc_examples/guide/request_data/query_params/project-installation.snap"

If you're using the default [`ApiKit`](../../dependency_injection/core_concepts/kits.md), 
you don't need to register a constructor for [`QueryParams<T>`][QueryParams] manually:
it's already included in the kit.

## Overview

Let's keep using `/search?sorted=true` as an example.  

You can parse the value for `sorted` by injecting [`QueryParams<T>`][QueryParams] in your handler:

--8<-- "doc_examples/guide/request_data/query_params/project-extraction.snap"

There are a few moving parts here. Let's break them down!

### Fields names

[`QueryParams<T>`][QueryParams] is a generic wrapper around a struct that models the query parameters for a given path.  
All struct fields must be named after the query parameters you want to extract.

In our example, the query parameter is named `sorted`.  
Our extraction type, `SearchParams`, must have a matching field named `sorted`.

--8<-- "doc_examples/guide/request_data/query_params/project-struct.snap"

### Deserialization

The newly defined struct must be **deserializable**—i.e. it must implement the [`serde::Deserialize`][serde::Deserialize] trait.  
You can derive [`serde::Deserialize`][serde::Deserialize] in most cases.

--8<-- "doc_examples/guide/request_data/query_params/project-struct_with_attr.snap"

### Parsing

From a protocol perspective, all query parameters are strings.  
From an application perspective, you might want to enforce stricter constraints.

In our example, we expect the `sorted` parameter to be a boolean.  
We could set the field type for `sorted` to `String` and then parse it into a boolean in the handler; however, that's going
to get tedious if we need to do it every single time we want to work with a boolean query parameter.  
We can skip all that boilerplate by setting the field type to `bool` directly, and let Pavex do the parsing for us:

--8<-- "doc_examples/guide/request_data/query_params/project-typed_field.snap"

Everything works as expected because `bool` implements the [`serde::Deserialize`][serde::Deserialize] trait.

## Supported field types

All "value" types (booleans, numbers, strings, etc.) can be used as fields in your query struct
(i.e. the `T` in `QueryParams<T>`).  

### Sequences

There is no standard way to represent sequences in query parameters.  
Pavex supports the [form style](https://swagger.io/docs/specification/serialization/#query), as specified by OpenAPI:

```rust
#[derive(serde::Deserialize)]
pub struct SearchParams {
    // This will parse `?country_id=1&country_id=2&country_id=3`
    // into a vector `vec![1, 2, 3]`.  
    //
    // Pavex does not perform any pluralization, therefore you must use
    // `serde`'s rename attribute if you want to use a pluralized name
    // as struct field but a singularized name in the query string.
    #[serde(rename = "country_id")]
    country_ids: Vec<u32>
}
```

Another common way to represent sequences in query parameters is to use brackets.
E.g. `?country_ids[]=1&country_ids[]=2&country_ids[]=3`.

You can use the `serde`'s rename attribute to support the bracket style:

```rust
#[derive(serde::Deserialize)]
pub struct SearchParams {
    // This will parse `?country_ids[]=1&country_ids[]=2&country_ids[]=3`
    // into a vector `vec![1, 2, 3]`.  
    #[serde(rename = "country_ids[]")]
    country_ids: Vec<u32>
}
```

## Unsupported field types

[`QueryParams<T>`][QueryParams] doesn't support deserializing nested structures.
For example, the following can't be deserialized from the wire using [`QueryParams<T>`][QueryParams]:

```rust
#[derive(serde::Deserialize)]
pub struct SearchParams {
    address: Address
}

#[derive(serde::Deserialize)]
pub struct Address {
    street: String,
    city: String,
}
```

If you need to deserialize nested structures from query parameters,
you might want to look into writing your own extractor on top of [`serde_qs`](https://crates.io/crates/serde_qs).

## Avoiding allocations

If you want to squeeze out the last bit of performance from your application,
you can try to avoid heap memory allocations when extracting string-like query parameters.  
Pavex supports this use case—**you can borrow from the request's query**.

### Percent-encoding

It is not always possible to avoid allocations when handling query parameters.  
Query parameters must comply with the restriction of the URI specification:
you can only use [a limited set of characters](https://datatracker.ietf.org/doc/html/rfc3986#section-2).  
If you want to use a character not allowed in a URI, you must [percent-encode it](https://developer.mozilla.org/en-US/docs/Glossary/Percent-encoding).  
For example, if you want to use a space in a query parameter, you must encode it as `%20`.
A string like `John Doe` becomes `John%20Doe` when percent-encoded.

[`QueryParams<T>`][QueryParams] automatically decodes percent-encoded strings for you. But that comes at a cost:
Pavex _must_ allocate a new `String` if the route parameter is percent-encoded.

### Cow

We recommend using [`Cow<'_, str>`][Cow] as your field type for string-like parameters.
It borrows from the request's path if possible, it allocates a new `String` if it can't be avoided.

[`Cow<'_, str>`][Cow] strikes a balance between performance and robustness: you don't have to worry about a runtime error if the route parameter
is percent-encoded, but you tried to use `&str` as its field type.

[QueryParams]: ../../../api_reference/pavex/request/query/struct.QueryParams.html
[QueryParams::register]: ../../../api_reference/pavex/request/query/struct.QueryParams.html#method.register
[serde::Deserialize]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
[Cow]: https://doc.rust-lang.org/std/borrow/enum.Cow.html
