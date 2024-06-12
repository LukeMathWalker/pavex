# Prebuilt types

A **prebuilt type** is a type that Pavex expects **you** to provide.  
Whenever your mark a type as prebuilt, you're telling Pavex: "I'll build 
this type on my own, and then pass an instance over to you".  
In particular, you'll be passing that instance to [`build_application_state`](application_state.md), the function that Pavex generates to build the [`ApplicationState`](application_state.md) type.

## Registration

To mark a type as prebuilt, you must invoke the [`prebuilt`][Blueprint::prebuilt] method on your [`Blueprint`][Blueprint]:

// TODO: code snippet

You must provide an [unambiguous path](../cookbook.md) to the type, wrapped in the [`t!`][t] macro.

!!! warning "t! vs f!"

    [`t!`][t] stands for "type". It isn't [`f!`][f], the macro used to register
    function-like components, like constructors or middlewares.  
    If you mix them up, Pavex will return an error.
    
## The signature changes

Whenever you mark a type as prebuilt, the signature of the code-generated
[`build_application_state`](application_state.md) function will change to include that type as an input parameter.  
You'll have to adjust the calling code in [your server crate](../../project_structure.md#the-server-crate) accordingly. 
This may be surprising at first, since you don't often touch the code inside [the server crate](../../project_structure.md#the-server-crate), but it's entirely expected. Don't worry: you just have to follow the compiler's suggestions to get back
on track.

!!! info "Immutability"

    The only crate you're **never** supposed to modify is the [server SDK crate](../../project_structure.md#the-server-sdk), the one that Pavex generates for you. 
    The [server crate](../../project_structure.md#the-server-crate), on the other hand, is yours to modify as you see fit.
    
## Lifecycle

If a prebuilt input is only needed to construct [singletons][Lifecycle::Singleton], it'll be discarded after [`build_application_state`](application_state.md) returns.

If it's needed to process requests (e.g. as an input for a middleware), it'll be added as a field to [`ApplicationState`](application_state.md). 
In this case, Pavex will treat it as a [singleton][Lifecycle::Singleton] and
require it to implement the [`Send`][Send] and [`Sync`][Sync] traits.

[Lifecycle::Singleton]: ../../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton
[Send]: https://doc.rust-lang.org/std/marker/trait.Send.html
[Sync]: https://doc.rust-lang.org/std/marker/trait.Sync.html
[t]: ../../../api_reference/pavex/macro.t.html
[f]: ../../../api_reference/pavex/macro.f.html
[Blueprint::prebuilt]: ../../../api_reference/pavex/blueprint/struct.Blueprint.html#method.prebuilt.html
[Blueprint]: ../../../api_reference/pavex/blueprint/struct.Blueprint.html
