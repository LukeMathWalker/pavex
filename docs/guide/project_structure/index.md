# Project structure

As you have seen in the [Quickstart](/getting_started/quickstart/index.md) tutorial,
`pavex new` is a quick way to scaffold a new project and start working on it.
If you execute

```bash
pavex new demo
```

the CLI will create a project with the following structure:

```text
--8<-- "doc_examples/quickstart/demo-project_structure.snap"
```

What is the purpose of all those folders? Why is [`cargo-px`][cargo-px] needed to build a Pavex project?
Are there any conventions to follow?

This guide will answer all these questions and more.

## Summary

If you're in a hurry, here's a quick summary of the most important points:

- A Pavex project is a [Cargo workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html)
  with at least three crates:
  - [a core crate](app.md) (_library_), conventionally named `app`
  - [a server SDK crate](server_sdk.md) (_library_), conventionally named `server_sdk`
  - [a server crate](server.md) (_binary_), conventionally named `server`
- The `app` crate contains the [`Blueprint`][Blueprint] for your API. It's where you'll spend most of your time.
- The `server_sdk` crate is generated from the core crate by `pavex generate`, which is invoked automatically
  by [`cargo-px`][cargo-px] when building or running the project.\
  **You'll never modify `server_sdk` manually**.
- The `server` crate is the entrypoint for your application.
  You'll have to change it whenever the [application state changes](/guide/dependency_injection/application_state.md)
  or if you want to tweak the binary entrypoint (e.g. modify the default telemetry setup).
  Your integration tests live in this crate.

Using the `demo` project as an example, the relationship between the project crates can be visualised as follows:

```mermaid
graph 
  d[app] -->|contains| bp[Blueprint];
  bp -->|is used to generate| dss[server_sdk];
  dss -->|is used by| ds[server];
  dss -->|is used by| dst[API tests in server];
```

If you want to know more, read on!

[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[cargo-px]: https://github.com/LukeMathWalker/cargo-px
