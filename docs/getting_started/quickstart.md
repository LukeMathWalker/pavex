# Quickstart

!!! note "Estimated time: 20 minutes"

!!! warning "Prerequisites"

    Make sure you've installed all the [required tools](../getting_started/index.md) before starting
    this tutorial.

## Create a new Pavex project

The `pavex` CLI provides a `new` subcommand to scaffold a new Pavex project.
Let's use it to create a new project called `demo`:

```bash
pavex new demo && cd demo
```

## Commands

### Build a Pavex project

`cargo` is not enough, on its own, to build a Pavex project:
you need to use the [`cargo-px`](https://github.com/LukeMathWalker/cargo-px) subcommand instead(1).  
From a usage perspective, it's a **drop-in replacement for `cargo`**:
you can use it to build, test, run, etc. your project just like you would with `cargo` itself.
{ .annotate }

1. `cargo-px` is a thin wrapper around `cargo` that adds support for more powerful code generation,
   overcoming some limitations of `cargo`'s build scripts.

Let's use it to check that your project compiles successfully:

```bash
cargo px check # (1)!
```

1. `cargo px check` is faster than `cargo px build` because it doesn't produce an executable binary.
   It's the quickest way to check that your project compiles while you're working on it.

If everything went well, try to execute the test suite:

```bash
cargo px test
```

### Run a Pavex project

Now launch your application:

```bash
cargo px run
```

Once the application is running, you should start seeing JSON logs in your terminal:

```json
{
  "name": "demo",
  "msg": "Starting to listen for incoming requests at 127.0.0.1:8000",
  "level": 30,
  "target": "api"
  // [...]
}
```

Leave it running in the background and open a new terminal window.

### Issue your first request

Let's issue your first request to a Pavex application!    
The template project comes with a `GET /api/ping` endpoint to be used as health check.
Let's hit it:

```bash
curl -v http://localhost:8000/api/ping # (1)!
```

1. We are using curl here, but you can replace it with your favourite HTTP client!

If all goes according to plan, you'll receive a `200 OK` response with an empty body:

```text
> GET /api/ping HTTP/1.1
> Host: localhost:8000
> User-Agent: [...]
> Accept: */*
>
< HTTP/1.1 200 OK
< content-length: 0
< date: [...]
```

You've just created a new Pavex project, built it, launched it and verified that it accepts requests correctly.  
It's a good time to start exploring the codebase!

## Blueprint

The core of a Pavex project is its [`Blueprint`][Blueprint].  
It's the type you'll use to define your API: routes, middlewares, error handlers, etc.

You can find the [`Blueprint`][Blueprint] for the `demo` project in the `demo/src/blueprint.rs` file:

--8<-- "doc_examples/quickstart/demo-blueprint_definition.snap"

## Routing

### Route registration

All the routes exposed by your API must be registered with its [`Blueprint`][Blueprint].  
In the snippet below you can see the registration of the `GET /api/ping` route, the one you targeted with your `curl`
request.

--8<-- "doc_examples/quickstart/demo-route_registration.snap"

It specifies:

- The HTTP method (`GET`)
- The path (`/api/ping`)
- The fully qualified path to the handler function (`crate::routes::status::ping`), wrapped in a macro (`f!`)

### Request handlers

The `ping` function is the handler for the `GET /api/ping` route:

--8<-- "doc_examples/quickstart/demo-ping_handler.snap"

It's a public function that returns a [`StatusCode`][StatusCode].  
[`StatusCode`][StatusCode] is a valid response type for a Pavex handler since it implements
the [`IntoResponse`][IntoResponse] trait:
the framework
knows how to convert it into a "full" [`Response`][Response] object.

### Add a new route

The `ping` function is fairly boring: it doesn't take any arguments, and it always returns the same response.  
Let's spice things up with a new route: `GET /api/greet/:name`.  
It takes a dynamic **route parameter** (`name`) and we want it to return a success response with `Hello, {name}` as its
body.

Create a new module, `greet.rs`, in the `demo/src/routes` folder:

--8<-- "doc_examples/quickstart/02-new_submodule.snap"

--8<-- "doc_examples/quickstart/02-route_def.snap"

The body of the `greet` handler is stubbed out with `todo!()` for now, but we'll fix that soon enough.  
Let's register the new route with the [`Blueprint`][Blueprint] in the meantime:

--8<-- "doc_examples/quickstart/02-register_new_route.snap"

1. Dynamic route parameters are prefixed with a colon (`:`).

### Extract route parameters

To access the `name` route parameter from your new handler you must use the [`RouteParams`][RouteParams] extractor:

--8<-- "doc_examples/quickstart/03-route_def.snap"

1. The name of the field must match the name of the route parameter as it appears in the path we registered with
   the [`Blueprint`][Blueprint].
2. The [`RouteParams`][RouteParams] extractor is generic over the type of the route parameters.  
   In this case, we're using the `GreetParams` type we just defined.

You can now return the expected response from the `greet` handler:

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

## Dependency injection

You just added a new input parameter to your `greet` handler and, somehow, the framework was able to provide its value
at runtime without you having to do anything.  
How does that work?

It's all thanks to **dependency injection**.  
Pavex automatically injects the expected input parameters when invoking your handler functions as long as
it knows how to construct them.

### Constructor registration

Let's zoom in on [`RouteParams`][RouteParams]: how does the framework know how to construct it?  
You need to go back to the [`Blueprint`][Blueprint] to find out:

--8<-- "doc_examples/quickstart/04-register_common_invocation.snap"

The `register_common_constructors` function takes care of registering constructors for a set of types that
are defined in the `pavex` crate itself and commonly used in Pavex applications.
If you check out its definition, you'll see that it registers a constructor for [`RouteParams`][RouteParams]:

--8<-- "doc_examples/quickstart/04-route_params_constructor.snap"

It specifies:

- The fully qualified path to the constructor method, wrapped in a macro (`f!`)
- The constructor's lifecycle ([`Lifecycle::RequestScoped`](Lifecycle::RequestScoped)): the framework will invoke this
  constructor at most once per
  request

### A new extractor: `UserAgent`

There's no substitute for hands-on experience, so let's design a brand-new constructor for our demo project to
get a better understanding of how they work.  
We only want to greet people who include a `User-Agent` header in their request(1).
{ .annotate }

1. It's an arbitrary requirement, follow along for the sake of the example!

Let's start by defining a new `UserAgent` type:

--8<-- "doc_examples/quickstart/05-new_submodule.snap"

--8<-- "doc_examples/quickstart/05-user_agent.snap"

### Missing constructor

What if you tried to inject `UserAgent` into your `greet` handler straight away? Would it work?  
Let's find out!

--8<-- "doc_examples/quickstart/05-inject.snap"

1. New input parameter!

If you try to build the project now, you'll get an error from Pavex:

```text
--8<-- "doc_examples/quickstart/05-error.snap"
```

Pavex cannot do miracles, nor does it want to: it only knows how to construct a type if you tell it how to do so.

By the way: this is also your first encounter with Pavex's error messages!  
We strive to make them as helpful as possible. If you find them confusing, report it as a bug!

### Add a new constructor

To inject `UserAgent` into our `greet` handler, you need to define a constructor for it.  
Constructors, just like request handlers, can take advantage of dependency injection: they can request input parameters
that will be injected by the framework at runtime.  
Since you need to look at headers, ask for [`RequestHead`][RequestHead] as input parameter: the incoming request data,
minus the body.

```rust title="demo/src/user_agent.rs" hl_lines="10 11 12 13 14 15 16 17 18 19"
--8<-- "doc_examples/quickstart/06/demo/src/user_agent.rs"
```

Now register the new constructor with the [`Blueprint`][Blueprint]:

```rust title="demo/src/blueprint.rs" hl_lines="5 6 7 8"
--8<-- "doc_examples/quickstart/06/demo/src/blueprint.rs"
    // [...]
}
```

[`Lifecycle::RequestScoped`][Lifecycle::RequestScoped] is the right choice for this type: the data in `UserAgent` is
request-specific.  
You don't want to share it across requests ([`Lifecycle::Singleton`][Lifecycle::Singleton]) nor do you want to recompute
it multiple times for
the same request ([`Lifecycle::Transient`][Lifecycle::Transient]).

Make sure that the project compiles successfully now.

## Error handling

In `UserAgent::extract` you're only handling the happy path:
the method panics if the `User-Agent` header is not valid UTF-8.  
Panicking for bad user input is poor behavior: you should handle the issue gracefully and return an error instead.

Let's change the signature of `UserAgent::extract` to return a `Result` instead:

```rust title="demo/src/user_agent.rs"
--8<-- "doc_examples/quickstart/07/demo/src/user_agent.rs"
// [...]

--8<-- "doc_examples/quickstart/07/demo/src/user_agent.rs"
```

1. `ToStrError` is the error type returned by `to_str` when the header value is not valid UTF-8.

### All errors must be handled

If you try to build the project now, you'll get an error from Pavex:

```text
--8<-- "doc_examples/quickstart/07-error.snap"
```

Pavex is complaining: you can register a fallible constructor, but you must also register an error handler for it.

### Add an error handler

An error handler must convert a reference to the error type into a [`Response`][Response] (1).  
It decouples the detection of an error from its representation on the wire: a constructor doesn't need to know how the
error will be represented in the response, it just needs to signal that something went wrong.  
You can then change the representation of an error on the wire without touching the constructor: you only need to change
the
error handler.
{ .annotate }

1. Error handlers, just like request handlers and constructors, can take advantage of dependency injection!
   You could, for example, change the response representation according to the `Accept` header specified in the request.

Define a new `invalid_user_agent` function in `demo/src/user_agent.rs`:

```rust title="demo/src/user_agent.rs"
// [...]
--8<-- "doc_examples/quickstart/08/demo/src/user_agent.rs"
```

Then register the error handler with the [`Blueprint`][Blueprint]:

```rust title="demo/src/blueprint.rs" hl_lines="9"
--8<-- "doc_examples/quickstart/08/demo/src/blueprint.rs"
    // [...]
}
```

The application should compile successfully now.

## Testing

All your testing, so far, has been manual: you've been launching the application and issuing requests to it with `curl`.
Let's move away from that: it's time to write some automated tests!

### Black-box testing

The preferred way to test a Pavex application is to treat it as a black box: you should only test the application
through its HTTP interface. This is the most realistic way to test your application: it's how your users will
interact with it, after all.

The template project includes a reference example for the `/api/ping` endpoint:

```rust title="demo_server/tests/integration/ping.rs"
--8<-- "doc_examples/quickstart/09/demo_server/tests/integration/ping.rs"
```

1. `TestApi` is a helper struct that provides a convenient interface to interact with the application.  
   It's defined in `demo_server/tests/helpers.rs`.
2. `TestApi::spawn` starts a new instance of the application in the background.
3. `TestApi::get_ping` issues an actual `GET /api/ping` request to the application.

### Add a new integration test

Let's write a new integration test to verify the behaviour on the happy path for `GET /api/greet/:name`:

```rust title="demo_server/tests/integration/main.rs hl_lines="1"
--8<-- "doc_examples/quickstart/09/demo_server/tests/integration/main.rs"
```

```rust title="demo_server/tests/integration/greet.rs"
--8<-- "doc_examples/quickstart/09/demo_server/tests/integration/greet.rs"
```

It follows the same pattern as the `ping` test: it spawns a new instance of the application, issues a request to it
and verifies that the response is correct.  
Let's complement it with a test for the unhappy path as well: requests with a malformed `User-Agent` header should be
rejected.

```rust title="demo_server/tests/integration/greet.rs"
// [...]
--8<-- "doc_examples/quickstart/10/demo_server/tests/integration/greet.rs"
```

`cargo px test` should report three passing tests now. As a bonus exercise, try to add a test for the case where the
`User-Agent` header is missing.

## Going further

Your first (guided) tour of Pavex ends here: you've touched the key concepts of the framework and got some hands-on
experience with a basic application.  
From here onwards, you are free to carve out your own learning path: you can explore the rest of the documentation
to learn more about the framework, or you can start hacking on your own project, consulting the documentation on a
need-to-know basis.

[Blueprint]: ../api_reference/pavex/blueprint/struct.Blueprint.html

[StatusCode]: ../api_reference/pavex/http/struct.StatusCode.html

[Response]: ../api_reference/pavex/response/struct.Response.html

[IntoResponse]: ../api_reference/pavex/response/trait.IntoResponse.html

[RouteParams]: ../api_reference/pavex/request/route/struct.RouteParams.html

[Response::ok]: ../api_reference/pavex/response/struct.Response.html#method.ok

[set_typed_body]: ../api_reference/pavex/response/struct.Response.html#method.set_typed_body

[Lifecycle::Singleton]: ../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton

[Lifecycle::RequestScoped]: ../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.RequestScoped

[Lifecycle::Transient]: ../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Transient

[RequestHead]: ../api_reference/pavex/request/struct.RequestHead.html