# Json

You can use [`JsonBody<T>`][JsonBody] to work with [JSON-encoded](https://www.json.org/json-en.html) request bodies.  
[`JsonBody<T>`][JsonBody] parses the raw JSON into an instance of the type `T` you specified.

The request body is buffered in memory before being deserialized.

## Registration

If you're using the default [`ApiKit`][ApiKit],
you don't need to register a constructor for [`JsonBody`][JsonBody]:
it's already included in the kit.

If you're not using [`ApiKit`][ApiKit], you need to register a constructor for [`JsonBody<T>`][JsonBody].
You can use [`JsonBody::register`][JsonBody::register] to register the default constructor
and error handler:

--8<-- "doc_examples/guide/request_data/json/project-installation.snap"

1. You also need to register a constructor for [`BufferedBody`][BufferedBody]!  
   Check out the [BufferedBody guide](../byte_wrappers.md) for more details.

## Extraction 

Inject [`JsonBody<T>`][JsonBody] as an input in your components to access the parsed body:

--8<-- "doc_examples/guide/request_data/json/project-extraction.snap"

## Deserialization

The newly defined struct must be **deserializable**—i.e. it must implement the [`serde::Deserialize`][serde::Deserialize] trait.  
You can derive [`serde::Deserialize`][serde::Deserialize] in most cases.

--8<-- "doc_examples/guide/request_data/json/project-struct_with_attr.snap"

## Avoiding allocations

If you want to minimize memory usage, you can try to avoid unnecessary heap memory allocations when deserializing 
string-like fields from the body of the incoming request.
Pavex supports this use case—**you can borrow from the request body**.

### Escape sequences

It is not always possible to avoid allocations, though.
In particular,
Pavex must allocate a new `String` if the JSON string you are trying to deserialize contains escape sequences,
such as `\n` or a `\"`.
Using a `&str` in this case would result in a runtime error when attempting the deserialization.

### Cow

We recommend using [`Cow<'_, str>`][Cow] as your field type for string-like parameters.
It borrows from the request's body if possible, it allocates a new `String` if it can't be avoided.

[`Cow<'_, str>`][Cow] strikes a balance between performance and robustness: you don't have to worry about a runtime error 
if the field contains escape sequences, but you tried to use `&str` as its field type.

[BufferedBody]: ../../../../api_reference/pavex/request/body/struct.BufferedBody.html
[JsonBody]: ../../../../api_reference/pavex/request/body/struct.JsonBody.html
[JsonBody::register]: ../../../../api_reference/pavex/request/body/struct.JsonBody.html#method.register
[serde::Deserialize]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
[Cow]: https://doc.rust-lang.org/std/borrow/enum.Cow.html
[ApiKit]: ../../../dependency_injection/core_concepts/kits.md