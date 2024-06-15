# Project setup

!!! warning "Prerequisites"

    Make sure you've installed all the [required tools](../index.md) before starting
    this tutorial.

## Create a new Pavex project

The `pavex` CLI provides a `new` subcommand to scaffold a new Pavex project.  
You can choose between different templates, each one tailored for a specific use case. We'll use the `quickstart` template for this tutorial:

```bash
pavex new --template="quickstart" demo && cd demo
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
