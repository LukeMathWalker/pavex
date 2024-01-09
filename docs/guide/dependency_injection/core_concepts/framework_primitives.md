# Framework primitives

You don't have to register a constructor for every type you want to inject.  
Pavex provides a few types, called **framework primitives**, that you can inject
without having to register a constructor for them.

The framework primitives are:

- [`RequestHead`][RequestHead]. The incoming request data, minus the body.
- [`RawIncomingBody`][RawIncomingBody]. The raw body of the incoming request.
- [`RawPathParams`][RawPathParams]. The raw path parameters extracted from the incoming request.
- [`AllowedMethods`][AllowedMethods]. The HTTP methods allowed for the current request path.

They represent raw data from the incoming request ([`RequestHead`][RequestHead], [`RawIncomingBody`][RawIncomingBody])
or information coming from the routing system ([`AllowedMethods`][AllowedMethods], [`RawPathParams`][RawPathParams]).

## Convenient, but inflexible

As a [design philosophy](../../../overview/why_pavex.md), Pavex strives to be **flexible**.
You should be allowed to customize the framework to your needs, without having to fight against it
or having to give up significant functionality.  
In particular, you should be able to change the way a certain type is constructed, even if that
type is defined in the `pavex` crate. For example, you might want to change the JSON deserializer used to parse the incoming request body
and produce a [`JsonBody<T>`][JsonBody] instance.  
You lose this flexibility with framework primitives: you can't customize how they are constructed.
That's why we try to keep their number to a minimum.

[RequestHead]: ../../../api_reference/pavex/request/struct.RequestHead.html
[RawPathParams]: ../../../api_reference/pavex/request/path/struct.RawPathParams.html
[AllowedMethods]: ../../../api_reference/pavex/router/enum.AllowedMethods.html
[RawIncomingBody]: ../../../api_reference/pavex/request/body/struct.RawIncomingBody.html
[JsonBody]: ../../../api_reference/pavex/request/body/struct.JsonBody.html
