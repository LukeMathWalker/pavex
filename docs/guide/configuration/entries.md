# New entries

Configuration in Pavex is managed via your application's [`Blueprint`][Blueprint].

## Registration

Invoke the [`Blueprint::config`][Blueprint::config] method to add a new configuration entry:

--8<-- "doc_examples/guide/configuration/project-registration.snap"

[`config`][Blueprint::config] expects:

- a [unique configuration key](#configuration-key).
- an [unambiguous path](/guide/dependency_injection/cookbook.md) to the type, wrapped in the [`t!`][t] macro.

!!! warning "t! vs f!"

    [`t!`][t] stands for "type". It isn't [`f!`][f], the macro used to register
    function-like components, like constructors or middlewares.  
    If you mix them up, Pavex will return an error.

## Required Traits

Configuration types must implement the [`Debug`][Debug], [`Clone`][Clone], and [`serde::Deserialize`][Deserialize] traits.
[`serde::Deserialize`][Deserialize], in particular, is required to parse configuration values out of environment variables, configuration files,
and other sources.

--8<-- "doc_examples/guide/configuration/project-derives.snap"

## Configuration key

The configuration key uniquely identifies a configuration entry. It must start with a letter and can contain letters, digits and underscores.

Every configuration entry becomes a field in [`ApplicationConfig`](application_config.md), and the configuration key is used as the field name.
The key determines the expected configuration schema—e.g. [which environment variables can be used to
set its value](loading.md#environment-variables) and the structure of configuration files.

## Lifecycle

Configuration entries are treated as [singletons][Lifecycle::Singleton] from Pavex's [dependency injection system][DI].

Most configuration entries are only needed to construct other [singletons][Lifecycle::Singleton],
so they'll be discarded after [`ApplicationState::new`][ApplicationState] returns.

If a configuration entry is needed at runtime (e.g. to configure the behaviour of middleware or a request handler),
it'll be added as a field to [`ApplicationState`][ApplicationState].
In this case, Pavex will expect the configuration type to implement the [`Send`][Send] and [`Sync`][Sync] traits in addition
to the other [trait requirements](#required-traits).

[Lifecycle::Singleton]: /api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton
[ApplicationState]: /guide/dependency_injection/application_state.md
[Send]: https://doc.rust-lang.org/std/marker/trait.Send.html
[Sync]: https://doc.rust-lang.org/std/marker/trait.Sync.html
[t]: /api_reference/pavex/macro.t.html
[f]: /api_reference/pavex/macro.f.html
[Blueprint::config]: /api_reference/pavex/blueprint/struct.Blueprint.html#method.config
[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[server_crate]: /guide/project_structure/server.md
[Debug]: https://doc.rust-lang.org/std/fmt/trait.Debug.html
[Clone]: https://doc.rust-lang.org/std/clone/trait.Clone.html
[Deserialize]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
[DI]: /guide/dependency_injection/index.md
