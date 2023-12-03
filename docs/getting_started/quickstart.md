# Quickstart

!!! note "Estimated time: 10 minutes"

!!! warning "Prerequisites"

    Make sure you've installed all the [required tools](../getting_started/index.md) before starting
    this tutorial.

## Create a new Pavex project

The `pavex` CLI provides a `new` subcommand to scaffold a new Pavex project.
Let's use it to create a new project called `blog`:

```bash
pavex new blog && cd blog
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
  "name": "blog",
  "msg": "Starting to listen for incoming requests at 127.0.0.1:8000",
  "level": 30,
  "target": "api"
  // [...]
}
```
