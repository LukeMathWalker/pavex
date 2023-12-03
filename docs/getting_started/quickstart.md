# Quickstart

!!! note "Estimated time: 10 minutes"

!!! warning "Prerequisites"

    Make sure you've installed all the [required tools](../getting_started/index.md) before starting
    this tutorial.

## Create a new Pavex project

The `pavex` CLI provides a `new` subcommand to scaffold a new Pavex project.
Let's use it to create a new project called `demo`:

```bash
pavex new demo && cd demo
```

## Build a Pavex project

`cargo` is not enough, on its own, to build a Pavex project:
you need to use the [`cargo-px`](https://github.com/LukeMathWalker/cargo-px) subcommand instead (1).  
From a usage perspective, it's a **drop-in replacement for `cargo`**:
you can use it to build, test, run, etc. your project just like you would with `cargo` itself.
{ .annotate }

1.  `cargo-px` is a thin wrapper around `cargo` that adds support for more powerful code generation,
    overcoming some limitations of `cargo`'s build scripts.


Let's use it to build our project:

```bash
cargo px build
```

If everything went well, you can try to execute the test suite:

```bash
cargo px test
```

## Run a Pavex project

Let's launch our application:

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

## Issue your first request

Let's issue our first request to the API.  
The template project comes with a `GET /api/ping` endpoint to be used as health check.
Let's hit it!

```bash
# (1)!
curl -v http://localhost:8000/api/ping
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

## Blueprint: the manifest of a Pavex project

The core of a Pavex project is its `Blueprint`.  
It's the type you'll use to define your API: routes, middlewares, error handlers, etc.

You can find the `Blueprint` for the `demo` project in the `demo/src/blueprint.rs` file:

```rust title="demo/src/blueprint.rs"
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);

    add_telemetry_middleware(&mut bp);

    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp
}
```

## Route registration

All the routes exposed by your API must be registered with its `Blueprint`.  
In the snippet below you can see the registration of the `GET /api/ping` route, the one you targeted with your `curl` request.

```rust title="demo/src/blueprint.rs" hl_lines="7"
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);

    add_telemetry_middleware(&mut bp);

    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp
}
```

It specifies:

- The HTTP method (`GET`)
- The path (`/api/ping`)
- The fully qualified path to the handler function (`crate::routes::status::ping`), wrapped in a macro (`f!`)

## Request handlers

The `ping` function is the handler for the `GET /api/ping` route:

```rust title="demo/src/routes/status.rs"
use pavex::http::StatusCode;

/// Respond with a `200 OK` status code to indicate that the server is alive
/// and ready to accept new requests.
pub fn ping() -> StatusCode {
    StatusCode::OK
}
```

It's a public function that returns a `StatusCode`.  
`StatusCode` is a valid response type for a Pavex handler since it implements the `IntoResponse` trait: the framework
knows how to convert it into a "full" `Response` object.

## Dependency injection

The `ping` function is fairly boring: it doesn't take any arguments, and it always returns the same response.  
Let's spice things up with a new route: `GET /api/greet/:name`.  
It takes a dynamic **route parameter** (`name`) and we want it to return a success response with `Hello, {name}` as its body.

Create a new module, `greet.rs`, in the `demo/src/routes` folder:

```rust title="demo/src/routes/lib.rs" hl_lines="2"
pub mod status;
pub mod greet;
```

```rust title="demo/src/routes/greet.rs"
use pavex::response::Response;

pub fn greet() -> Response {
    todo!()
}
```

The body of the `greet` handler is stubbed out with `todo!()` for now, but we'll fix that soon enough.  
Let's register the new route with the `Blueprint` in the meantime:

```rust title="demo/src/blueprint.rs" hl_lines="8"
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);

    add_telemetry_middleware(&mut bp);

    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp.route(GET, "/api/greet/:name"/* (1)! */, f!(crate::routes::greet::greet));
    bp
}
```

1. Dynamic route parameters are prefixed with a colon (`:`).