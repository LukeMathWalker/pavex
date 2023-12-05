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

1.  `cargo-px` is a thin wrapper around `cargo` that adds support for more powerful code generation,
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

## Routing

### Route registration

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

### Request handlers

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

### Add a new route

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

### Extract route parameters

To access the `name` route parameter from your new handler you must use the `RouteParams` extractor:

```rust title="demo/src/routes/greet.rs"
use pavex::response::Response;
use pavex::request::RouteParams;

#[RouteParams]
pub struct GreetParams {
    pub name/* (1)! */: String,
}

pub fn greet(params: RouteParams<GreetParams>/* (2)! */) -> Response {
    todo!()
}
```

1. The name of the field must match the name of the route parameter as it appears in the path we registered with the `Blueprint`.
2. The `RouteParams` extractor is generic over the type of the route parameters.  
   In this case, we're using the `GreetParams` type we just defined.

You can now return the expected response from the `greet` handler:

```rust title="demo/src/routes/greet.rs" hl_lines="10 11 12 13"
use pavex::response::Response;
use pavex::request::route::RouteParams;

#[RouteParams]
pub struct GreetParams {
    pub name: String,
}

pub fn greet(params: RouteParams<GreetParams>) -> Response {
    let GreetParams { name }/* (1)! */= params.0; 
    Response::ok()// (2)!
        .set_typed_body(format!("Hello, {name}!"))// (3)!
        .box_body()
}
```

1. This is an example of Rust's [destructuring syntax](https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#destructuring-to-break-apart-values).
2. `Response` has a convenient constructor for each HTTP status code: `Response::ok()` starts building a `Response` with a `200 OK` status code.
3. `typed_body` sets the body of the response and automatically infers a suitable value for the `Content-Type` header based on the response body type.

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

Let's zoom in on `RouteParams`: how does the framework know how to construct it?  
You need to go back to the `Blueprint` to find out:

```rust title="demo/src/blueprint.rs" hl_lines="3"
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);

    add_telemetry_middleware(&mut bp);

    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp.route(GET, "/api/greet/:name", f!(crate::routes::greet::greet));
    bp
}
```

The `register_common_constructors` function takes care of registering constructors for a set of types that 
are defined in the `pavex` crate itself and commonly used in Pavex applications.
If you check out its definition, you'll see that it registers a constructor for `RouteParams`:

```rust title="pavex/src/blueprint.rs" hl_lines="3 4 5 6"
fn register_common_constructors(bp: &mut Blueprint) {
    // [...]
    bp.constructor(
        f!(pavex::request::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    // [...]
}
```

It specifies:

- The fully qualified path to the constructor method, wrapped in a macro (`f!`)
- The constructor's lifecycle (`Lifecycle::RequestScoped`): the framework will invoke this constructor at most once per request

### A new extractor: `UserAgent`

There's no substitute for hands-on experience, so let's design a brand-new constructor for our demo project to
get a better understanding of how they work.  
We only want to greet people who include a `User-Agent` header in their request(1).
{ .annotate }

1. It's an arbitrary requirement, follow along for the sake of the example!

Let's start by defining a new `UserAgent` type:

```rust title="demo/src/lib.rs"
//! [...]
pub mod user_agent;
```

```rust title="demo/src/user_agent.rs"
pub enum UserAgent {
    /// No `User-Agent` header was provided.
    Unknown,
    /// The value of the `User-Agent` header for the incoming request.
    Known(String),
}
```

### Missing constructor

What if you tried to inject `UserAgent` into your `greet` handler straight away? Would it work?  
Let's find out!

```rust title="demo/src/routes/greet.rs" hl_lines="4"
use crate::user_agent::UserAgent;
// [...]

pub fn greet(params: RouteParams<GreetParams>, user_agent: UserAgent/* (1)! */) -> Response {
    if let UserAgent::Anonymous = user_agent {
        return Response::unauthorized()
            .set_typed_body("You must provide a `User-Agent` header")
            .box_body();
    }
    // [...]
}
```

1. New input parameter!
   
If you try to build the project now, you'll get an error from Pavex:

```text
ERROR:
  × I can't invoke your request handler, `demo::routes::greet::greet`, because it needs an instance of
  │ `demo::user_agent::UserAgent` as input, but I can't find a constructor for that type.
  │
  │     ╭─[demo/src/blueprint.rs:13:1]
  │  13 │     bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
  │  14 │     bp.route(GET, "/api/greet/:name", f!(crate::routes::greet::greet));
  │     ·                                       ───────────────┬───────────────
  │     ·                                   The request handler was registered here
  │  15 │     bp
  │     ╰────
  │     ╭─[demo/src/routes/greet.rs:9:1]
  │   9 │
  │  10 │ pub fn greet(params: RouteParams<GreetParams>, _user_agent: UserAgent) -> Response {
  │     ·                                                             ────┬────
  │     ·                                              I don't know how to construct an instance 
  │     ·                                                    of this input parameter
  │  11 │     let GreetParams { name } = params.0;
  │     ╰────
  │   help: Register a constructor for `demo::user_agent::UserAgent`
```

Pavex cannot do miracles, nor does it want to: it only knows how to construct a type if you tell it how to do so.

By the way: this is also your first encounter with Pavex's error messages!  
We strive to make them as helpful as possible. If you find them confusing, report it as a bug!

### Add a new constructor

To inject `UserAgent` into our `greet` handler, you need to define a constructor for it.  
Constructors, just like request handlers, can take advantage of dependency injection: they can request input parameters
that will be injected by the framework at runtime.  
Since you need to look at headers, ask for `RequestHead` as input parameter: the incoming request data, 
minus the body.

```rust title="demo/src/user_agent.rs" hl_lines="10 11 12 13 14 15 16 17 18 19"
use pavex::http::header::USER_AGENT;
use pavex::request::RequestHead;

pub enum UserAgent {
    Unknown,
    Known(String),
}
    
impl UserAgent {
    pub fn extract(request_head: &RequestHead) -> Self {
        let Some(user_agent) = request_head.headers.get(USER_AGENT) else {
            return Self::Anonymous;
        };

        match user_agent.to_str() {
            Ok(s) => Self::Known(s.into()),
            Err(_e) => todo!()
        }
    }
}
```

Now register the new constructor with the `Blueprint`:

```rust title="demo/src/blueprint.rs" hl_lines="5 6 7 8"
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);

    bp.constructor(
        f!(crate::user_agent::UserAgent::extract),
        Lifecycle::RequestScoped,
    );
    // [...]
}
```

`Lifecycle::RequestScoped` is the right choice for this type: the data in `UserAgent` is request-specific.  
You don't want to share it across requests (`Lifecycle::Singleton`) nor do you want to recompute it multiple times for 
the same request (`Lifecycle::Transient`).  

Make sure that the project compiles successfully now.

## Error handling

In `UserAgent::extract` you're only handling the happy path:
the method panics if the `User-Agent` header is not valid UTF-8.  
Panicking for bad user input is poor behavior: you should handle the issue gracefully and return an error instead.

Let's change the signature of `UserAgent::extract` to return a `Result` instead:

```rust title="demo/src/user_agent.rs"
use pavex::http::header::{ToStrError, USER_AGENT};
// [...]

impl UserAgent {
    pub fn extract(request_head: &RequestHead) -> Result<Self, ToStrError/* (1)! */> {
        let Some(user_agent) = request_head.headers.get(USER_AGENT) else {
            return Ok(UserAgent::Anonymous);
        };

        user_agent.to_str().map(|s| UserAgent::Known(s.into()))
    }
}
```

1. `ToStrError` is the error type returned by `to_str` when the header value is not valid UTF-8. 

### All errors must be handled

If you try to build the project now, you'll get an error from Pavex:

```text
ERROR:
  × You registered a constructor that returns a `Result`, but you did not register an error handler for it. 
  | If I don't have an error handler, I don't know what to do with the error when the constructor fails!
  │
  │     ╭─[demo/src/blueprint.rs:11:1]
  │  11 │     bp.constructor(
  │  12 │         f!(crate::user_agent::UserAgent::extract),
  │     ·         ────────────────────┬────────────────────
  │     ·                             ╰── The fallible constructor was registered here
  │  13 │         Lifecycle::RequestScoped,
  │     ╰────
  │   help: Add an error handler via `.error_handler`
```

Pavex is complaining: you can register a fallible constructor, but you must also register an error handler for it.  

### Add an error handler

An error handler must convert a reference to the error type into a `Response` (1).  
It decouples the detection of an error from its representation on the wire: a constructor doesn't need to know how the
error will be represented in the response, it just needs to signal that something went wrong.  
You can then change the representation of an error on the wire without touching the constructor: you only need to change the
error handler.
{ .annotate }

1. Error handlers, just like request handlers and constructors, can take advantage of dependency injection! 
   You could, for example, change the response representation according to the `Accept` header specified in the request.

Define a new `invalid_user_agent` function in `demo/src/user_agent.rs`:

```rust title="demo/src/user_agent.rs"
// [...]

pub fn invalid_user_agent(_e: &ToStrError) -> Response {
    Response::bad_request()
        .set_typed_body("The `User-Agent` header value must be a valid UTF-8 string")
        .box_body()
}
```

Then register the error handler with the `Blueprint`:

```rust title="demo/src/blueprint.rs" hl_lines="9"
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);

    bp.constructor(
        f!(crate::user_agent::UserAgent::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(crate::user_agent::invalid_user_agent));
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
use crate::helpers::TestApi;//(1)!
use pavex::http::StatusCode;

#[tokio::test]
async fn ping_works() {
    let api = TestApi::spawn().await;//(2)!

    let response = api.get_ping().await;//(3)!

    assert_eq!(response.status().as_u16(), StatusCode::OK.as_u16());
}
```

1. `TestApi` is a helper struct that provides a convenient interface to interact with the application.  
   It's defined in `demo_server/tests/helpers.rs`.
2. `TestApi::spawn` starts a new instance of the application in the background.
3. `TestApi::get_ping` issues an actual `GET /api/ping` request to the application.

### Add a new integration test

Let's write a new integration test to verify the behaviour on the happy path for `GET /api/greet/:name`:

```rust title="demo_server/tests/integration/main.rs hl_lines="1"
mod greet;
mod ping;
mod helpers;
```

```rust title="demo_server/tests/integration/greet.rs"
use crate::helpers::TestApi;
use pavex::http::StatusCode;

#[tokio::test]
async fn greet_happy_path() {
    let api = TestApi::spawn().await;
    let name = "Ursula";

    let response = api
        .api_client
        .get(&format!("{}/api/greet/{name}", &api.api_address))
        .header("User-Agent", "Test runner")
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), StatusCode::OK.as_u16());
    assert_eq!(response.text().await.unwrap(), "Hello, Ursula!");
}
```

It follows the same pattern as the `ping` test: it spawns a new instance of the application, issues a request to it
and verifies that the response is correct.  
Let's complement it with a test for the unhappy path as well: requests with a malformed `User-Agent` header should be rejected.

```rust title="demo_server/tests/integration/greet.rs"
// [...]
#[tokio::test]
async fn non_utf8_user_agent_is_rejected() {
    let api = TestApi::spawn().await;
    let name = "Ursula";

    let response = api
        .api_client
        .get(&format!("{}/api/greet/{name}", &api.api_address))
        .header("User-Agent", b"hello\xfa".as_slice())
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), StatusCode::BAD_REQUEST.as_u16());
    assert_eq!(
        response.text().await.unwrap(),
        "The `User-Agent` header value must be a valid UTF-8 string"
    );
}
```

`cargo px test` should report three passing tests now. As a bonus exercise, try to add a test for the case where the
`User-Agent` header is missing.

## Going further

Your first (guided) tour of Pavex ends here: you've touched the key concepts of the framework and got some hands-on
experience with a basic application.  
From here onwards, you are free to carve out your own learning path: you can explore the rest of the documentation 
to learn more about the framework, or you can start hacking on your own project, consulting the documentation on a
need-to-know basis.
