# `ApplicationState`

When generating the [server SDK crate], Pavex examines all the components you registered to determine which [singletons][Lifecycle::Singleton] will be
used at runtime to process incoming requests.\
Pavex then generates a type to group them together, named `ApplicationState`.

## `build_application_state`

Inside the [server SDK crate], you'll also find a function named [`build_application_state`][build_application_state]. As the name suggest, it returns an instance of `ApplicationState`.

`build_application_state` takes as input all the types that you marked
as [prebuilt](prebuilt_types.md).
Inside its body, it'll invoke the constructors for all your [singletons][Lifecycle::Singleton] in order to build an instance of `ApplicationState`.

[Lifecycle::Singleton]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton
[build_application_state]: ../project_structure.md#applicationstate
[server crate]: ../project_structure.md#the-server-crate
[ApplicationState]: ../project_structure.md#applicationstate
[server SDK crate]: ../project_structure.md#the-server-sdk
