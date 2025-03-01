# Installation

The session machinery is not included in the project scaffolded by `pavex new`.
You need to go through a few steps to set it up.

## Dependencies

The core session logic lives in a standalone crate, [`pavex_session`][pavex_session].\
Support for different session storage backends is provided by separate crates, such as
[`pavex_session_sqlx`][pavex_session_sqlx] and [`pavex_session_memory_store`][pavex_session_in_memory_store].

Choose a storage backend that fits your needs and then add the required dependencies to the `Cargo.toml`
of your application crate:

=== "Postgres"

    ```toml
    [dependencies]
    # [...]
    pavex_session = "0.1"
    pavex_session_sqlx = { version = "0.1", features = ["postgres"] }
    ```

=== "In-memory"

    ```toml
    [dependencies]
    # [...]
    pavex_session = "0.1"
    pavex_session_memory_store = "0.1"
    ```

## Kits

Kits bundles together all the components you need to work with a specific session setup.
Register the one provided by the storage backend you chose against your [`Blueprint`][Blueprint]:

=== "Postgres"

    --8<-- "doc_examples/guide/sessions/installation/project-postgres.snap"

=== "In-memory"

    --8<-- "doc_examples/guide/sessions/installation_memory/project-in_memory.snap"

You can customize each component inside the kit to suit your needs.
Check out their respective documentation for more information.

## `SessionConfig`

[`SessionConfig`][SessionConfig] determines how sessions are processed by your application.
What's the name of the session cookie? How long should it last? Do we create a
server-side session state for every client-side cookie?

The example above invokes `with_default_config` to rely on the default settings.\
If you wish to customize the session behaviour, follow these steps:

1. Add [`SessionConfig`][SessionConfig] as a field on your application's `AppConfig` struct, usually located
   in `app/src/configuration.rs`.
2. Register a constructor that returns a [`SessionConfig`][SessionConfig] instance by accessing the field.

Check out the "Realworld" example as a reference:

1. [Configuration field](https://github.com/LukeMathWalker/pavex/blob/310afce47413bbfc56ffa7a1b15940086ce7e773/examples/realworld/app/src/configuration.rs#L15)
2. [Constructor registration](https://github.com/LukeMathWalker/pavex/blob/310afce47413bbfc56ffa7a1b15940086ce7e773/examples/realworld/app/src/configuration.rs#L35)

[pavex_session]: ../../api_reference/pavex_session/index.html
[pavex_session_sqlx]: ../../api_reference/pavex_session_sqlx/index.html
[pavex_session_in_memory_store]: ../../api_reference/pavex_session_in_memory_store/index.html
[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[SessionConfig]: ../../api_reference/pavex_session/struct.SessionConfig.html
[default settings]: ../../api_reference/pavex_session/struct.SessionConfig.html#fields
[build_application_state]: ../project_structure.md#applicationstate
