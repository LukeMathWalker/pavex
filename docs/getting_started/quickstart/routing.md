# Routing

## Route registration

All the routes exposed by your API must be registered with its [`Blueprint`][Blueprint].  
In the snippet below you can see the registration of the `GET /api/ping` route, the one you targeted with your `curl`
request.

--8<-- "doc_examples/quickstart/demo-route_registration.snap"

It specifies:

- The HTTP method (`GET`)
- The path (`/api/ping`)
- An [unambiguous path](../../guide/dependency_injection/cookbook.md) to the handler function (`self::ping::get`), wrapped in the [`f!`][f!] macro

## Request handlers

The `ping` function is the handler for the `GET /api/ping` route:

--8<-- "doc_examples/quickstart/demo-ping_handler.snap"

It's a public function that returns a [`StatusCode`][StatusCode].  
[`StatusCode`][StatusCode] is a valid response type for a Pavex handler since it implements
the [`IntoResponse`][IntoResponse] trait:
the framework
knows how to convert it into a "full" [`Response`][Response] object.

## Add a new route

The `ping` function is fairly boring: it doesn't take any arguments, and it always returns the same response.  
Let's spice things up with a new route: `GET /api/greet/:name`.  
It takes a dynamic **route parameter** (`name`) and we want it to return a success response with `Hello, {name}` as its
body.

Create a new module, `greet.rs`, in the `app/src/routes` folder:

--8<-- "doc_examples/quickstart/02-new_submodule.snap"

--8<-- "doc_examples/quickstart/02-route_def.snap"

The body of the `GET /api/greet/:name` handler is stubbed out with `todo!()` for now, but we'll fix that soon enough.  
Let's register the new route with the [`Blueprint`][Blueprint] in the meantime:

--8<-- "doc_examples/quickstart/02-register_new_route.snap"

1. Dynamic path parameters are prefixed with a colon (`:`).

## Extract path parameters

To access the `name` route parameter from your new handler you must use the [`PathParams`][PathParams] extractor:

--8<-- "doc_examples/quickstart/03-route_def.snap"

1. The name of the field must match the name of the route parameter as it appears in the path we registered with
   the [`Blueprint`][Blueprint].
2. The [`PathParams`][PathParams] extractor is generic over the type of the path parameters.  
   In this case, we're using the `GreetParams` type we just defined.

You can now return the expected response from the handler:

--8<-- "doc_examples/quickstart/04-route_def.snap"

1. This is an example of
   Rust's [destructuring syntax](https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#destructuring-to-break-apart-values).
2. [`Response`][Response] has a convenient constructor for each HTTP status code: [`Response::ok`][Response::ok] starts
   building a [`Response`][Response] with
   a `200 OK` status code.
3. [`set_typed_body`][set_typed_body] sets the body of the response and automatically infers a suitable value for
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

[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html

[StatusCode]: ../../api_reference/pavex/http/struct.StatusCode.html

[Response]: ../../api_reference/pavex/response/struct.Response.html

[IntoResponse]: ../../api_reference/pavex/response/trait.IntoResponse.html

[PathParams]: ../../api_reference/pavex/request/path/struct.PathParams.html

[Response::ok]: ../../api_reference/pavex/response/struct.Response.html#method.ok

[set_typed_body]: ../../api_reference/pavex/response/struct.Response.html#method.set_typed_body

[f!]: ../../api_reference/pavex/macro.f!.html
