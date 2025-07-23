# Routing

## Route definition

[Routes][routes], like all Pavex components, are defined using [**attributes**][attributes].

--8<-- "docs/tutorials/quickstart/snaps/ping_route.snap"

1. [`#[pavex::get]`][pavex_get_attr] defines a route that responds to `GET` requests for the specified path.

## Route registration

It's not enough to _define_ a route, you also need to _register_ it with the [`Blueprint`][Blueprint].
That's taken care of by the `.routes()` invocation:

--8<-- "docs/tutorials/quickstart/snaps/route_import.snap"

It's registering, in bulk, all the routes defined in the current crate, including our `ping`. Since it introduces a prefix,
the final route is `GET /api/ping` rather than `GET /ping`.

## Add a new route

The `ping` function is fairly boring: it doesn't take any arguments, and it always returns the same response.

Let's spice things up with a new route: `GET /api/greet/{name}`.
Its path includes a [**dynamic path parameter**][path_parameters] (`name`) and we want it to return a success response with `Hello, {name}` as its body.

Create a new `greet` module under `routes` to hold the route definition:

--8<-- "docs/tutorials/quickstart/snaps/new_greet_mod.snap"

--8<-- "docs/tutorials/quickstart/snaps/greet_route_stub.snap"

1. [Dynamic path parameters][path_parameters] are enclosed in curly braces (`{}`).

The body of the `GET /api/greet/{name}` handler is stubbed out with `todo!()` for now, but we'll fix that soon enough.

## Extract path parameters

To access the `name` route parameter from your new handler you must use the [`PathParams`][PathParams] extractor:

--8<-- "docs/tutorials/quickstart/snaps/greet_route_impl.snap"

1. The name of the field must match the name of the route parameter as it appears in the path we registered with
   the [`Blueprint`][Blueprint].
2. The [`PathParams`][PathParams] extractor is generic over the type of the path parameters.
   In this case, we're using the `GreetParams` type we just defined.
3. This is an example of
   Rust's [destructuring syntax](https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#destructuring-to-break-apart-values).
4. [`Response`][Response] has a convenient constructor for each HTTP status code: [`Response::ok`][Response::ok] starts
   building a [`Response`][Response] with
   a `200 OK` status code.
5. [`set_typed_body`][set_typed_body] sets the body of the response and automatically infers a suitable value for
   the `Content-Type` header
   based on the response body type.

Does it work? Only one way to find out!
Re-launch the application and issue a new request: (1)
{ .annotate }

1. Remember to use `cargo px run` instead of `cargo run`!

```bash
curl http://localhost:8000/api/greet/Ursula
```

You should see `Hello, Ursula!` in your terminal if everything went well.

[attributes]: /guide/attributes/index.md
[routes]: /guide/routing/index.md
[pavex_get_attr]: /api_reference/pavex/attr.get.html
[path_parameters]: /guide/routing/path_patterns.md#path-parameters
[Blueprint]: /api_reference/pavex/struct.Blueprint.html
[StatusCode]: /api_reference/pavex/http/struct.StatusCode.html
[Response]: /api_reference/pavex/response/struct.Response.html
[IntoResponse]: /api_reference/pavex/response/trait.IntoResponse.html
[PathParams]: /api_reference/pavex/request/path/struct.PathParams.html
[Response::ok]: /api_reference/pavex/response/struct.Response.html#method.ok
[set_typed_body]: /api_reference/pavex/response/struct.Response.html#method.set_typed_body
[f!]: /api_reference/pavex/macro.f!.html
