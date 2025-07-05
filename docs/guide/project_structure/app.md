# App

The application crate is where most of your code lives.
In particular, it's where you define your [`Blueprint`][Blueprint].

!!! note

    As a convention, the application crate is named `app`.  
    You can use `{project_name}_app` if you need to disambiguate between
    multiple Pavex applications in the same workspace.

## Blueprint

Every Pavex project has, at its core, a [`Blueprint`][Blueprint].
It's the type you use to declare the structure of your API:
[routes], [middlewares], [constructors], [error handlers], [error observers], [configuration], etc.

--8<-- "docs/tutorials/quickstart/snaps/blueprint.snap"

Think of a [`Blueprint`][Blueprint] as the specification for your API, a **plan for how your application should behave at
runtime**.

But you can't run or execute a [`Blueprint`][Blueprint] as-is. That's where code generation comes in.

## Code generation

To convert a [`Blueprint`][Blueprint] into an executable toolkit, you need `pavex generate`.
It's a CLI command that takes a [`Blueprint`][Blueprint] as input and outputs a
Rust crate, the [**server SDK**](server_sdk.md) for your Pavex project.

### `cargo-px`

If you went through the [Quickstart](/getting_started/quickstart/index.md) tutorial, you might be
wondering: I've never run `pavex generate`! How comes my project worked?

That's thanks to [`cargo-px`][cargo-px]!\
If you look into the `Cargo.toml` manifest for the `server_sdk` crate in the `demo` project,
you'll find this section:

--8<-- "docs/tutorials/quickstart/snaps/generate_directive.snap"

It's a [`cargo-px`][cargo-px] configuration section.\
The `server_sdk` crate is telling [`cargo-px`][cargo-px] to generate the whole crate
by executing a binary called `bp` (short for `blueprint`) from the current Cargo workspace.

That binary is defined in the `demo` crate:

--8<-- "docs/tutorials/quickstart/snaps/generator_entrypoint.snap"

[`Client::generate`][Client::generate] takes care of serializing the [`Blueprint`][Blueprint]
and passing it as input to `pavex generate`.

All this is done automatically for you when you run `cargo px build` or `cargo px run`.
[`cargo-px`][cargo-px] examines all the crates in your workspace, generates the ones
that need it, and then goes on to complete the build process.

[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[Client::generate]: /api_reference/pavex_cli_client/client/struct.Client.html#method.generate
[Lifecycle::Singleton]: /api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton
[Server]: /api_reference/pavex/server/struct.Server.html
[routes]: ../routing/index.md
[constructors]: ../dependency_injection/index.md
[middlewares]: ../middleware/index.md
[error handlers]: ../errors/error_handlers.md
[error observers]: ../errors/error_observers.md
[configuration]: ../configuration/index.md
[cargo-px]: https://github.com/LukeMathWalker/cargo-px
