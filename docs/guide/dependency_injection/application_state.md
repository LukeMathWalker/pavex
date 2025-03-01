# `ApplicationState`

When generating the [server SDK crate], Pavex examines all the components you registered to determine which [singletons][Lifecycle::Singleton] will be
used at runtime to process incoming requests.\
Pavex then generates a type to group them together, named (unsurprisingly) `ApplicationState`.

## `ApplicationState::new`

You need to invoke the `ApplicationState::new` method to build an instance of `ApplicationState`.

`ApplicationState::new` takes as input `ApplicationConfig` and all the types that you marked
as [prebuilt](prebuilt_types.md).
Inside its body, it'll invoke the constructors for all your [singletons][Lifecycle::Singleton] in order to build an instance of `ApplicationState`.

[Lifecycle::Singleton]: /api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton
[server SDK crate]: ../project_structure/server_sdk.md
