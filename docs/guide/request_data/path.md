# Path

All incoming HTTP requests include a [request target](https://datatracker.ietf.org/doc/html/rfc7230#section-5.3)[^rfc].  
The target usually[^rfc] takes the form of a URI, either in absolute form (e.g. `https://example.com/foo/bar?baz=qux`) or in
origin form (e.g. `/foo/bar?baz=qux`).

## URI components

The **path** is the component of a URI that comes after the authority and before the query.  
Let's take `https://example.com/foo/bar?baz=qux` as an example:

- The **scheme** is `https`.
- The **authority** is `example.com`.
- The **path** is `/foo/bar`.
- The **query** is `baz=qux`.

If the request target is in origin form, the authority and the scheme are omitted: you're left with just the path and the query,
e.g. `/foo/bar?baz=qux`.

## Raw path

You can access the raw path, as is, by injecting [`RequestHead`][RequestHead] and accessing its [`uri`][RequestHead::uri] field:

--8<-- "doc_examples/guide/request_data/path/project-raw_path.snap"

## Route parameters

In REST APIs, the path is often used to identify a resource.  
For example, in `https://example.com/users/123`, the path is `/users/123` and the resource is the user with ID `123`.

Those dynamic path segments are called **route parameters**.  
In Pavex, you must declare the route parameters for a given path in the route definition—see [Route parameters](../routing/path_patterns.md#route-parameters) 
for more details.

Let's keep using `https://example.com/users/123` as an example.  
To extract `123` from the path, you register `/users/:id` as the path pattern for that route.  

--8<-- "doc_examples/guide/request_data/path/project-route_params_registration.snap"

1. The path pattern for the route.

You can then access the `id` value for an incoming request by injecting [`RouteParams<T>`][RouteParams] in your handler:

--8<-- "doc_examples/guide/request_data/path/project-route_params_extraction.snap"

There's a lot going on here, so let's break it down!

### Fields names

[`RouteParams<T>`][RouteParams] is a generic wrapper around a struct[^why-struct] that contains the route parameters for a given path.  
All struct fields must be named after the route parameters declared in the path pattern[^wrong-name].

In our example, the path pattern is `/users/:id`.
We defined a new struct for its route parameters, `GetUserParams`, with a single field named `id`.

--8<-- "doc_examples/guide/request_data/path/project-route_params_struct.snap"

### Deserialization

The newly defined struct must be **deserializable**—i.e. it must implement the [`serde::Deserialize`][serde::Deserialize] trait.  
The [`#[RouteParams]`][RouteParamsMacro] attribute macro will automatically derive [`serde::Deserialize`][serde::Deserialize] for you. Alternatively, you can derive or implement [`serde::Deserialize`][serde::Deserialize] directly.  

--8<-- "doc_examples/guide/request_data/path/project-route_params_struct_with_attr.snap"

If you rely on [`#[RouteParams]`][RouteParamsMacro], Pavex can perform more advanced checks at compile time[^structural-deserialize] (e.g. detect unsupported types).

### Parsing

From a protocol perspective, all route parameters are strings.  
From an application perspective, you might want to enforce stricter constraints.

In our example, we expect `id` parameter to be a number.  
We could set the field type for `id` to `String` and then parse it into a number in the handler; however, that's going
to get tedious if we need to do it every single time we want to work with a numeric route parameter.  
We can skip all that boilerplate by setting the field type to `u64` directly, and let Pavex do the parsing for us:

--8<-- "doc_examples/guide/request_data/path/project-route_params_typed_field.snap"

Everything works as expected because `u64` implements the [`serde::Deserialize`][serde::Deserialize] trait.

### Unsupported types

Route parameters are best used to encode **values**, such as numbers, strings, or dates.  
There is no standard way to encode more complex types such as collections (e.g. `Vec<T>`, tuples) in a route parameter. 
As a result, Pavex doesn't support them.

Pavex will do its best to catch unsupported types at compile time, but it's not always possible. 

### Avoiding allocations

If you want to squeeze out the last bit of performance from your application,
you can try to avoid memory allocations when extracting string-like route parameters.  
Pavex supports this use case—**you can borrow from the request's path**.

#### Percent-encoding

It is not always possible to avoid allocations when handling route parameters.  
Route parameters must comply with the restriction of the URI specification:
you can only use [a limited set of characters](https://datatracker.ietf.org/doc/html/rfc3986#section-2).  
If you want to use a character not allowed in a URI, you must [percent-encode it](https://developer.mozilla.org/en-US/docs/Glossary/Percent-encoding).  
For example, if you want to use a space in a route parameter, you must encode it as `%20`.
A string like `John Doe` becomes `John%20Doe` when percent-encoded.

[`RouteParams<T>`][RouteParams] automatically decodes percent-encoded strings for you. But that comes at a cost: 
Pavex _must_ allocate a new `String` if the route parameter is percent-encoded.

#### Cow

We recommend using [`Cow<'_, str>`][Cow] as your field type for string-like parameters.
It borrows from the request's path if possible, it allocates a new `String` if it can't be avoided.

[`Cow<'_, str>`][Cow] strikes a balance between performance and robustness: you don't have to worry about a runtime error if the route parameter
is percent-encoded, but you tried to use `&str` as its field type.


[^rfc]: [RFC 7230](https://datatracker.ietf.org/doc/html/rfc7230#section-5.3) allows for two other formats of request target,
    authority form (e.g. `example.com:443`) and asterisk form (e.g. `*`), but they're rarely relevant for 
    the HTTP APIs you're likely to build with Pavex.  
    Nonetheless, you can work with all kinds of request targets in Pavex by accessing [`RequestHead::uri`][RequestHead::uri]
    directly.

[^why-struct]: Pavex made a deliberate choice of _not_ supporting tuples or other sequence-like types for extracting route parameters. 
    Check out [the API reference](../../api_reference/pavex/request/route/struct.RouteParams.html#unsupported-types)
    to learn more about the rationale behind this decision.

[^wrong-name]: If a field name doesn't match a route parameter name, Pavex will detect it at compile time and return
    an error.
    No more runtime errors because you misspelled a field name!

[^structural-deserialize]: Check the documentation for [`StructuralDeserialize`][StructuralDeserialize] if you want
    to know more about the underlying mechanism.

[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
[RequestHead::uri]: ../../api_reference/pavex/request/struct.RequestHead.html#structfield.uri
[RouteParams]: ../../api_reference/pavex/request/route/struct.RouteParams.html
[RouteParamsMacro]: ../../api_reference/pavex/request/route/attr.RouteParams.html
[serde::Deserialize]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
[StructuralDeserialize]: ../../api_reference/pavex/serialization/trait.StructuralDeserialize.html
[Cow]: https://doc.rust-lang.org/std/borrow/enum.Cow.html