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

## Blueprint

You need to add a few imports and middlewares to your [`Blueprint`][Blueprint] to get sessions up and running:

=== "Postgres"

    --8<-- "docs/examples/sessions/postgres/postgres.snap"

    1. `pavex` provides the core request/response types as well as cookies.
    2. `pavex_session` provides the `Session` type and the machinery to manage the session lifecycle.
    3. `pavex_session_sqlx::postgres` provides a Postgres-based session store implementation.
    4. [`finalize_session`][finalize_session] looks at the current session state and decides whether 
       a session cookie must be set or not on the outgoing response.
    5. [`inject_response_cookies`][inject_response_cookies] converts [`ResponseCookies`][ResponseCookies]
       into `Set-Cookie` headers on the response.\
       It **must** execute after [`finalize_session`][finalize_session],
       otherwise the session cookie will not be set.
       If you get the order wrong, the code generation process will fail.

=== "In-memory"

    --8<-- "docs/examples/sessions/in_memory/in_memory.snap"

    1. `pavex` provides the core request/response types as well as cookies.
    2. `pavex_session` provides the `Session` type and the machinery to manage the session lifecycle.
    3. `pavex_session_memory_store` provides the in-memory session store implementation.
    4. [`finalize_session`][finalize_session] looks at the current session state and decides whether 
       a session cookie must be set or not on the outgoing response.
    5. [`inject_response_cookies`][inject_response_cookies] converts [`ResponseCookies`][ResponseCookies]
       into `Set-Cookie` headers on the response.\
       It **must** execute after [`finalize_session`][finalize_session],
       otherwise the session cookie will not be set.
       If you get the order wrong, the code generation process will fail.

Sessions are built on top of [cookies][cookie], so both must be installed for sessions to work correctly.

[cookie]: /guide/cookies/index.md
[pavex_session]: /api_reference/pavex_session/index.html
[pavex_session_sqlx]: /api_reference/pavex_session_sqlx/index.html
[pavex_session_memory_store]: /api_reference/pavex_session_memory_store/index.html
[ResponseCookies]: /api_reference/pavex/cookie/struct.ResponseCookies.html
[inject_response_cookies]: /api_reference/pavex/cookie/fn.inject_response_cookies.html
[finalize_session]: /api_reference/pavex_session/fn.finalize_session.html
[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
