# Dependency injection

You just added a new input parameter to your `greet` handler and, somehow, the framework was able to provide its value
at runtime without you having to do anything.  
How does that work?

It's all thanks to [**dependency injection**](../../guide/dependency_injection/index.md).  
Pavex automatically injects the expected input parameters when invoking your handler functions as long as
it knows how to construct them.

## Constructor registration

Let's zoom in on [`PathParams`][PathParams]: how does the framework know how to construct it?  
You need to go back to the [`Blueprint`][Blueprint] to find out:

--8<-- "doc_examples/quickstart/04-register_common_invocation.snap"

[`ApiKit`][ApiKit] is one of Pavex's [kits](../../guide/dependency_injection/core_concepts/kits.md): it
bundles together [constructors](../../guide/dependency_injection/core_concepts/constructors.md) for types
that are commonly used when building APIs with Pavex.  
In particular, it includes a constructor for [`PathParams`][PathParams].  

## A new extractor: `UserAgent`

Kits give you a head start, but they're not the last stop on your journey: to leverage Pavex to 
its full potential, you'll soon need to define and register your own constructors.  
There's no substitute for hands-on experience: let's design together a brand-new constructor 
for our demo project to get a better understanding of how it all works.  
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


In [`Blueprint::constructor`][Blueprint::constructor] you must specify:

- The [fully qualified path](../../guide/dependency_injection/cookbook.md) to the constructor method, wrapped in a macro ([`f!`][f!])
- The constructor's lifecycle ([`Lifecycle::RequestScoped`](Lifecycle::RequestScoped))

[`Lifecycle::RequestScoped`][Lifecycle::RequestScoped] is the right choice for this type: the data in `UserAgent` is
request-specific.
Using [`Lifecycle::RequestScoped`][Lifecycle::RequestScoped] ensures that the framework will invoke the constructor
at most once per request.  
You don't want to share it across requests ([`Lifecycle::Singleton`][Lifecycle::Singleton]) nor do you want to recompute
it multiple times for
the same request ([`Lifecycle::Transient`][Lifecycle::Transient]).

Make sure that the project compiles successfully now.

[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::constructor]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.constructor
[f!]: ../../api_reference/pavex/macro.f!.html
[PathParams]: ../../api_reference/pavex/request/path/struct.PathParams.html
[ApiKit]: ../../api_reference/pavex/kit/struct.ApiKit.html

[Lifecycle::Singleton]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Singleton

[Lifecycle::RequestScoped]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.RequestScoped

[Lifecycle::Transient]: ../../api_reference/pavex/blueprint/constructor/enum.Lifecycle.html#variant.Transient

[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
