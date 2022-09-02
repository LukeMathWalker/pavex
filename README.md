# `pavex`

_< Insert cool logo here >_

## Project status

`pavex` is under active development and far from being ready for hobby or production usage.  
It has not yet been released on crates.io and you can expect breaking changes on every commit to the `main` branch.

`pavex` is publicly available, today, because I love to build in public.  
It also makes it easier for me, as a maintainer, to share code snippets and get other people's help
on some of the gnarliest parts of the project.

## What is `pavex`?

`pavex` is a source code generator for building APIs and web applications with Rust.  
It takes as input a high-level description of what your application should do.
It generates as output the source code for a fully-fleshed web-server, behaving according to your specification, ready
to be launched.

```rust
use pavex_builder::{f, AppBlueprint, Lifecycle};
use pavex_runtime::{Request, Body, Response};
use std::path::PathBuf;

/// The blueprint for our application.
/// It lists all its routes and provides constructors for all the types
/// that will be needed to invoke `stream_file`, our request handler.
///
/// This will be turned into a ready-to-run web server by `pavex_cli`.
pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::load_configuration), Lifecycle::Singleton)
        .constructor(f!(crate::http_client), Lifecycle::Singleton)
        .constructor(f!(crate::extract_path), Lifecycle::RequestScoped)
        .constructor(f!(crate::logger), Lifecycle::Transient)
        .route(f!(crate::stream_file), "/home")
}

pub fn stream_file(
    inner: PathBuf,
    logger: Logger,
    http_client: reqwest::Client,
) -> Response<Body> { /* */ }

pub fn extract_path(request: Request<Body>) -> PathBuf { /* */ }

pub fn logger() -> Logger { /* */ }

pub fn load_configuration() -> Config { /* */ }

pub fn http_client(config: Config) -> reqwest::Client { /* */ }
```

## Why does `pavex` exist?

`actix-web`, `rocket`, `axum`, `tide`, `warp` - we have plenty of web frameworks in the Rust ecosystem, even
limiting the list to the most popular ones.

Why `pavex`? Why would you go and build yet another web framework?

To broaden the design space!  
I believe there is an under-explored opportunity to significantly **improve the developer experience** of Rust web
developers **by raising the level of abstraction of their tools**.

The current generation of Rust web frameworks is trying to walk a tight rope.
On one side, they strive to provide ergonomic APIs, lowering the bar for more and more people to get started building
APIs in Rust.
On the other side, they want to provide high-performance and (wherever possible) misuse-resistant interfaces with
compile-time guarantees of correctness.

There is tension between those two objectives.  
High-performance and compile-safety drives frameworks to lean heavily on the expressiveness of Rust's type systems,
trying to encode invariants
for compile-time verification as well as limiting the overhead of the framework itself.  
This all works just fine on the happy path, but it can lead to obscure compiler errors on the unhappy path - often
too obscure for beginners trying to make sense of what is happening.

Let's look at `axum`'s "Hello world" example as a case study:

```rust
use axum::{response::Html, routing::get, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(handler));
    axum::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 3000)))
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
```

It may happen that a developer forgets to mark `handler` as `async` - to be fair, it doesn't really need to be `async`
given that there is no `.await` in its body.

```rust
// [...]
// No longer `async`!
fn handler() -> Html<&'static str> { /* */ }
```

The compiler greets us with this error message:

```text
error[E0277]: the trait bound `fn() -> Html<&'static str> {handler}: Handler<_, _, _>` is not satisfied
   --> hello-world/src/main.rs:12:44
    |
12  |     let app = Router::new().route("/", get(handler));
    |                                        --- ^^^^^^^ 
                                             |   the trait `Handler<_, _, _>` is not implemented 
                                             |   for `fn() -> Html<&'static str> {handler}`
    |                                        |
    |                                        required by a bound introduced by this call
    |
    = help: the trait `Handler<T, S, B>` is implemented for `Layered<L, H, T, S, B>`
note: required by a bound in `axum::routing::get`
   --> /Users/luca/code/axum/axum/src/routing/method_routing.rs:400:1
    |
400 | top_level_handler_fn!(get, GET);
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `axum::routing::get`
    = note: this error originates in the macro `top_level_handler_fn`
```

Good luck figuring that out!  
Especially if you are at the beginning of your journey with Rust.

Is the situation helpless? Are we forced to choose between performance, compile-time safety and ergonomics?

No, we are not.  
There are ongoing efforts inside the Rust project to improve diagnostics further and empower crate authors to "suggest"
to the compiler appropriate error messages in specific situations (check
out [this PR](https://github.com/bevyengine/bevy/pull/5786) in `bevy`!).

At the same time, crate authors are trying to step in with the tools currently available in Rust's latest stable
release: metaprogramming.  
`axum` provides a `#[debug_handler]` procedural macro that can be used to annotate request handlers. It has no effect on
runtime
behaviour, but
it allows `axum` to preempt the compiler and emit _its own compiler errors_ when it detects certain patterns of
incorrect behaviour.

Let's use the "Hello world" example again to try it out:

```rust
// [...]
// Now annotated with #[debug_handler], same sync signature otherwise
#[axum::debug_handler]
fn handler() -> Html<&'static str> { /* */ }
```

The error message is now **much** better:

```text
error: handlers must be async functions
  --> main.rs:xx:1
   |
xx | fn handler() -> &'static str {
   | ^^
```

This is **amazing**, because it is speaking **at the right level of abstraction**.  
It is talking about handlers, a concept that we understand as API developers. A concept that we had just tried to use.  
No type noise, no need to look at the open guts of `axum`'s inner abstractions.

This is where the idea for `pavex` comes from.  
What if we took this metaprogramming approach to the next level?  
Let's get rid of most of the user-facing complexity - heavily generic APIs, intricate trait bounds, nested type chains.
We will use straight-forward Rust to specify what your application should do.  
We will then feed our app specification to `pavex_cli`, a transpiler designed _specifically_ for web applications.  
It will generate the source code for our web server, using (once again) straight-forward Rust. If something goes wrong,
it will return **meaningful errors that speak the language of web applications**.

You might wonder - is it even feasible? And you'd be right to doubt!  
We need to overcome significant technical challenges to build a tool that lives up to the vision laid out above.
Guess what: it's fun to try!

## Architectural Overview

If the section above was enough to get you intrigued, you can check out the architectural deep-dive
in [`ARCHITECTURE.md`](ARCHITECTURE.md) to learn how `pavex` works under the hood.

## Contributing

This project is not open to unsolicited code contributions (for the time being).  
If you want to play around with it, you can find instructions in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as
defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
