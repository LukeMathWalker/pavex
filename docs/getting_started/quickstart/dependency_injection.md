# Dependency injection

You just added a new input parameter to your `greet` handler and, somehow, the framework was able to provide its value
at runtime without you having to do anything.  
How does that work?

It's all thanks to **dependency injection**.  
Pavex automatically injects the expected input parameters when invoking your handler functions as long as
it knows how to construct them.

## Constructor registration

Let's zoom in on [`PathParams`][PathParams]: how does the framework know how to construct it?  
You need to go back to the [`Blueprint`][Blueprint] to find out:

--8<-- "doc_examples/quickstart/04-register_common_invocation.snap"

The `register_common_constructors` function takes care of registering constructors for a set of types that
are defined in the `pavex` crate itself and commonly used in Pavex applications.
If you check out its definition, you'll see that it registers a constructor for [`PathParams`][PathParams]:

--8<-- "doc_examples/quickstart/04-route_params_constructor.snap"

Inside [`PathParams::register`][PathParams::register] you'll find:

```rust
use crate::blueprint::constructor::{Constructor, Lifecycle};
use crate::blueprint::Blueprint;
use crate::f;

impl PathParams<()> {
    pub fn register(bp: &mut Blueprint) -> Constructor {
        bp.constructor(
            f!(pavex::request::path::PathParams::extract),
            Lifecycle::RequestScoped,
        )
        .error_handler(f!(
            pavex::request::path::errors::ExtractPathParamsError::into_response
        ))
    }
}
```

It specifies:

- The [fully qualified path](../../guide/dependency_injection/cookbook.md) to the constructor method, wrapped in a macro (`f!`)
- The constructor's lifecycle ([`Lifecycle::RequestScoped`](Lifecycle::RequestScoped)): the framework will invoke this
  constructor at most once per
  request

## A new extractor: `UserAgent`

There's no substitute for hands-on experience, so let's design a brand-new constructor for our demo project to
get a better understanding of how they work.  
We only want to greet people who include a `User-Agent` header in their request(1).
{ .annotate }

1. It's an arbitrary requirement, follow along for the sake of the example!

Let's start by defining a new `UserAgent` type:

--8<-- "doc_examples/quickstart/05-new_submodule.snap"

--8<-- "doc_examples/quickstart/05-user_agent.snap"

## Missing constructor

What if you tried to inject `UserAgent` into your `greet` handler straight away? Would it work?  
Let's find out!

--8<-- "doc_examples/quickstart/05-inject.snap"

1. New input parameter!

If you try to build the project now, you'll get an error from Pavex:

```ansi-color
--8<-- "doc_examples/quickstart/05-error.snap"
```

Pavex cannot do miracles, nor does it want to: it only knows how to construct a type if you tell it how to do so.

By the way: this is also your first encounter with Pavex's error messages!  
We strive to make them as helpful as possible. If you find them confusing, report it as a bug!

## Add a new constructor

To inject `UserAgent` into our `greet` handler, you need to define a constructor for it.  
Constructors, just like request handlers, can take advantage of dependency injection: they can request input parameters
that will be injected by the framework at runtime.  
Since you need to look at headers, ask for [`RequestHead`][RequestHead] as input parameter: the incoming request data,
minus the body.

--8<-- "doc_examples/quickstart/06-extract.snap"

Now register the new constructor with the [`Blueprint`][Blueprint]:

--8<-- "doc_examples/quickstart/06-register.snap"

[`Lifecycle::RequestScoped`][Lifecycle::RequestScoped] is the right choice for this type: the data in `UserAgent` is
request-specific.  
You don't want to share it across requests ([`Lifecycle::Singleton`][Lifecycle::Singleton]) nor do you want to recompute
it multiple times for
the same request ([`Lifecycle::Transient`][Lifecycle::Transient]).

Make sure that the project compiles successfully now.

[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html

[PathParams]: ../../api_reference/pavex/request/path/struct.PathParams.html
[PathParams::register]: ../../api_reference/pavex/request/path/struct.PathParams.html#method.register

[Lifecycle::Singleton]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton

[Lifecycle::RequestScoped]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.RequestScoped

[Lifecycle::Transient]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Transient

[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
