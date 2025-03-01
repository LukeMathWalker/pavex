# Installation

The session machinery is not included in the project scaffolded by `pavex new`.
You need to go through a few steps to set it up.

## Dependencies

The core session logic lives in a standalone crate, [`pavex_session`][pavex_session].\
Support for different session storage backends is provided by separate crates, such as
[`pavex_session_sqlx`][pavex_session_sqlx] and [`pavex_session_memory_store`][pavex_session_memory_store].

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

[pavex_session]: /api_reference/pavex_session/index.html
[pavex_session_sqlx]: /api_reference/pavex_session_sqlx/index.html
[pavex_session_memory_store]: /api_reference/pavex_session_memory_store/index.html
[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
