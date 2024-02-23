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

In [`Blueprint::request_scoped`][Blueprint::request_scoped] you must specify 
the [fully qualified path](../../guide/dependency_injection/cookbook.md) to the constructor method, wrapped in a macro ([`f!`][f!]).

Make sure that the project compiles successfully at this point.

## Lifecycles

A constructor registered via [`Blueprint::request_scoped`][Blueprint::request_scoped] has
a **[request-scoped lifecycle][lifecycle]**: the framework
will invoke a request-scoped constructor **at most once per request**.

You can register constructors with two other lifecycles: **[singleton][lifecycle]**
and **[transient][lifecycle]**.  
Singletons are built once and shared across requests.  
Transient constructors, instead, are invoked every time their output type is needed—potentially
multiple times for the same request.

`UserAgent` wouldn't be a good fit as a singleton or a transient constructor:

- `UserAgent` depends on the headers of the incoming request.
  It would be incorrect to mark it as a singleton and share it across requests 
  (and Pavex wouldn't allow it!)
- You _could_ register `UserAgent` as transient, but extracting (and parsing) the `User-Agent` header
  multiple times would be wasteful.
  As a request-scoped constructor, it's done once and the outcome is reused.

[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[Blueprint::request_scoped]: ../../api_reference/pavex/blueprint/struct.Blueprint.html#method.request_scoped
[f!]: ../../api_reference/pavex/macro.f!.html
[PathParams]: ../../api_reference/pavex/request/path/struct.PathParams.html
[ApiKit]: ../../api_reference/pavex/kit/struct.ApiKit.html
[lifecycle]: ../../guide/dependency_injection/core_concepts/constructors.md#lifecycles
[RequestHead]: ../../api_reference/pavex/request/struct.RequestHead.html
