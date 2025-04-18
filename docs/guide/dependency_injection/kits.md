# Kits

Pavex provides a [rich set of first-party constructors](../request_data/index.md).\
To leverage them, you must register them with your application's [`Blueprint`][Blueprint]: after a while,
it gets tedious.
To make your life easier, Pavex provides **kits**: collections of commonly used constructors,
organized by application type.

## `ApiKit`

`ApiKit` is a good starting point for most APIs and it's installed by default in projects created with
`pavex new`.

--8<-- "doc_examples/guide/dependency_injection/kit/project-default_kit.snap"

It registers constructors for:

- [`PathParams`][PathParams]
- [`QueryParams`][QueryParams]
- [`BufferedBody`][BufferedBody]
- [`BodySizeLimit`][BodySizeLimit]
- [`JsonBody`][JsonBody]

## Customization

Using a kit it's not an all-or-nothing deal: you can cherry-pick the constructors you need and
even customize them.

### Skip a constructor

If you don't need a particular constructor, you can skip its registration by setting the corresponding
kit field to `None`:

--8<-- "doc_examples/guide/dependency_injection/kit/project-skip.snap"

### Tweak a constructor

In other cases, you may want to include a constructor, but the default configuration doesn't fit your requirements.\
For example, you might want to change the cloning behavior or the associated error handler.

--8<-- "doc_examples/guide/dependency_injection/kit/project-tweak.snap"

### Replace a constructor

You can also replace one of the constructors provided by the kit with a custom one.

--8<-- "doc_examples/guide/dependency_injection/kit/project-replace.snap"

1. When working with a kit,
   you configure the constructor _without_ registering it directly with the blueprint.\
   The kit takes care of the registration for you when its `register` method is invoked.

[PathParams]: ../request_data/path/path_parameters.md
[QueryParams]: ../request_data/query/query_parameters.md
[BufferedBody]: ../request_data/body/byte_wrappers.md
[BodySizeLimit]: ../request_data/body/byte_wrappers.md#body-size-limit
[JsonBody]: ../request_data/body/deserializers/json.md
[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
