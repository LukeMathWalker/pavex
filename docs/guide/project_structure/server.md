# Server

The [server SDK] crate is a library, it doesn't contain an executable binary.
That's why you need a **server crate**.

!!! note

    As a convention, the server crate is named `server`.  
    You can use `{project_name}_server` if you need to disambiguate between
    multiple Pavex applications in the same workspace.

## The executable binary

The server crate contains the `main` function that you'll be running to start your application.\
In that `main` function you'll be building an instance of [`ApplicationState`](server_sdk.md#applicationstate) and passing it to `run`.
You'll be doing a few other things too: initializing your `tracing` subscriber, loading
configuration, etc.

??? info "The `main` function in `server`"

    --8<-- "docs/tutorials/quickstart/snaps/server_entrypoint.snap"

Most of this ceremony is taken care for you by the `pavex new` command, but it's good to know
that it's happening (and where it's happening) in case you need to customize it.

## Integration tests

The server crate is also where you'll be writing your **API tests**, also known as **black-box tests**.\
These are scenarios that exercise your application as a customer would, by sending HTTP requests and asserting on the
responses.

The `demo` project includes an example of such a test which you can use as a reference:

--8<-- "docs/tutorials/quickstart/snaps/ping_test.snap"

[server SDK]: server_sdk.md
