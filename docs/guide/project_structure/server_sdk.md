# Server SDK

The server SDK is the glue that wires everything together. It is the code
executed at runtime when a request hits your API.

It is generated by Pavex, following the instructions in your [`Blueprint`][Blueprint].

!!! note

    As a convention, the generated crate is named `server_sdk`.  
    You can use `{project_name}_server_sdk` if you need to disambiguate between
    multiple Pavex applications in the same workspace.

## Don't touch the generated code

You can think of the server SDK crate as the output of a macro: to modify the outcome,
you need to modify the input (i.e. the blueprint).\
Don't modify the generated code directly—any changes you make will be overwritten
the next time you run `cargo px build`, `cargo px run` or any other command that triggers
the (re)generation of the server SDK crate.

The server SDK crate has an advantage over regular macro-generated code: you can explore it!
It's right there in your filesystem: you can open it, you can read it, you can use it as a way
to get a deeper understanding of how Pavex works under the hood.

## Key items

As a Pavex user, you don't need to read the generated code (unless you are curious, of course).
You'll only interact with the three public items that the server SDK crate exports: `run`, [`ApplicationState`](#applicationstate)
and [`ApplicationConfig`](#applicationconfig).

## `ApplicationConfig`

[`ApplicationConfig`](/guide/configuration/index.md) specifies all the configuration options for your application.\
Each field in `ApplicationConfig` corresponds to a configuration type you registered with the application blueprint.

Configuration is loaded from environment variables and configuration files as soon as the application starts,
in the [server crate](server.md).

## `ApplicationState`

[`ApplicationState`](/guide/dependency_injection/application_state.md) is the global state of your application.
`ApplicationState` is instantiated before the application starts listening for incoming requests and sticks around
until the application is shut down.

## `run`

When you invoke `run`, the HTTP server starts listening for incoming requests. You're live!\
`run` takes as input [`ApplicationState`](#applicationstate) and [`pavex::server::Server`][Server].
[`Server`][Server] holds the HTTP server configuration: the port(s) to listen on,
the number of worker threads to be used, etc.\

But who is in charge of invoking `run`? The `main` function in the [server crate](server.md)!

[Blueprint]: /api_reference/pavex/struct.Blueprint.html
[Server]: /api_reference/pavex/server/struct.Server.html
