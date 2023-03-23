# `pavex`

> The goal is simple: great ergonomics and high performance - no sacrifices.
> As easy to use as `tide`, Rails or ASP.NET Core.
> As fast as a handwritten solution built directly on top of raw `hyper`.

_< Insert cool logo here >_

## What is `pavex`?

`pavex` is a source code generator for building APIs and web applications with Rust.  
It takes as input a high-level description of what your application should do.
It generates as output the source code for a fully-fleshed web-server, behaving according to your specification, ready
to be launched.

It aims to deliver **high performance** _as well as_ an **amazing developer experience**.

Check out the [announcement blog post](https://www.lpalmieri.com/posts/a-taste-of-pavex-rust-web-framework/) for more
details on the vision.

You can see `pavex` at work in the [`/examples` folder](./examples):

- In [`examples/app_blueprint/src/lib.rs`](./examples/app_blueprint/src/lib.rs) we specify the app's behavior in
  a `Blueprint`—
  the endpoints it exposes and their request handlers, as well as the required constructors for the application state;
- In [`examples/app_blueprint/src/bin.rs`](./examples/app_blueprint/src/bin.rs) we serialize the `Blueprint` and
  invoke `pavex`'s CLI to generate the server code that will execute at runtime, which you can find in
  [`examples/generated_app/src/lib.rs`](./examples/generated_app/src/lib.rs).

In [`examples/app_blueprint/blueprint.ron`](./examples/app_blueprint/blueprint.ron) you can have a peek at what
the `Blueprint` looks like when serialized.

## Project status

`pavex` is under active development and far from being ready for hobby or production usage.  
It has not yet been released on crates.io and you can expect breaking changes on every commit to the `main` branch.

We publish project updates every 4-6 weeks:

- [Progress report #1](https://www.lpalmieri.com/posts/pavex-progress-report-01/)
- [Progress report #2](https://www.lpalmieri.com/posts/pavex-progress-report-02/)

## Why does `pavex` exist?

Check out the [announcement blog post](https://www.lpalmieri.com/posts/a-taste-of-pavex-rust-web-framework/) for an
overview of the vision.

## Architectural Overview

If the section above was enough to get you intrigued, you can check out the architectural deep-dive
in [`ARCHITECTURE.md`](ARCHITECTURE.md) to learn how `pavex` works under the hood.

## Contributing

This project is not open to unsolicited code contributions (for the time being).  
If you want to play around with it, you can find instructions in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

Licensed under the Apache License, Version 2.0.
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as
defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.
