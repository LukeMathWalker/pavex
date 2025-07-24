# Dependency injection

You just added a new input parameter to your request handler and, somehow, the framework was able to provide its value
at runtime without you having to do anything.
How does that work?

It's all thanks to [**dependency injection**](../../guide/dependency_injection/index.md).\
Pavex automatically injects the expected input parameters when invoking your handler functions as long as
it knows how to construct them.

## Constructor registration

Let's zoom in on [`PathParams`][PathParams]: how does the framework know how to construct it?\
You need to go back to the [`Blueprint`][Blueprint] to find out:

--8<-- "docs/tutorials/quickstart/snaps/pavex_import.snap"

We're importing all the constructors defined in the `pavex` crate.
In particular, this includes a constructor for [`PathParams`][PathParams].

## A new extractor: `UserAgent`

The framework gives you a head start with its built-in components, but they're not enough: to build a real application with Pavex, you'll soon need to define and register your own constructors.
There's no substitute for hands-on experience: let's design together a brand-new constructor
for our demo project to get a better understanding of how it all works.

We will only greet people who include a `User-Agent` header in their request[^arbitrary].

Let's start by defining a new `UserAgent` type:

--8<-- "docs/tutorials/quickstart/snaps/user_agent_mod.snap"

--8<-- "docs/tutorials/quickstart/snaps/user_agent_def.snap"

## Missing constructor

What if you tried to inject `UserAgent` into your request handler straight away? Would it work?\
Let's find out!

--8<-- "docs/tutorials/quickstart/snaps/greet_agent_input.snap"

1. New input parameter!

If you try to build the project now, you'll get an error from Pavex:

```ansi-color
--8<-- "docs/tutorials/quickstart/snaps/missing_user_agent_constructor.snap"
```

Pavex cannot do miracles, nor does it want to: it only knows how to construct a type if you tell it how to do so.

By the way: this is also your first encounter with Pavex's error messages!\
We strive to make them as helpful as possible. If you find them confusing, file a bug report.

## Add a new constructor

To inject `UserAgent` into your request handler, you need to define a constructor for it.\
Constructors, just like request handlers, can take advantage of dependency injection: they can request input parameters
that will be injected by the framework at runtime.\
Since you need to look at headers, ask for [`RequestHead`][RequestHead] as input parameter: the incoming request data,
minus the body.

--8<-- "docs/tutorials/quickstart/snaps/user_agent_extract.snap"

The `#[request_scoped]` annotation tells Pavex that the new method is a **constructor**.\

Try to recompile the project—there should be no error now.\
The new constructor was picked up immediately because our [`Blueprint`][Blueprint]
is configured to import all constructors defined in the current crate:

--8<-- "docs/tutorials/quickstart/snaps/import_crate.snap"

## Lifecycles

A constructor registered via [`#[request_scoped]`][request_scoped] has
a **[request-scoped lifecycle][lifecycle]**: the framework
will invoke a request-scoped constructor **at most once per request**.

You can register constructors with two other lifecycles: **[singleton][lifecycle]**
and **[transient][lifecycle]**.\
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

[Blueprint]: /api_reference/pavex/struct.Blueprint.html
[request_scoped]: /api_reference/pavex/attr.request_scoped.html
[PathParams]: /api_reference/pavex/request/path/struct.PathParams.html
[lifecycle]: ../../guide/dependency_injection/constructors.md#lifecycles
[RequestHead]: /api_reference/pavex/request/struct.RequestHead.html

[^arbitrary]: It's an arbitrary requirement, follow along for the sake of the example!
