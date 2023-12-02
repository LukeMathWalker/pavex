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

The project scaffolded by `pavex new` bundles a hierarchical configuration system: you can load
different configuration values for different **profiles**.  
The `APP_PROFILE` environment variable tells the application which profile to use.

Let's run our application in `development` mode:

```bash
APP_PROFILE=dev cargo px run --bin api
```

After the build is complete, you should see the following output:
