# Domain guards

A **domain guard** restricts a group of routes to a specific domain.\
With domain guards you can serve multiple websites and/or APIs from the same Pavex application.

--8<-- "doc_examples/guide/routing/domain_guards/project-intro.snap"

## Static guards

The simplest case is a static domain, a domain guard that matches a single, predetermined domain:

--8<-- "doc_examples/guide/routing/domain_guards/project-static.snap"

It will only match requests to `pavex.dev`.
It won't match, for example, requests to `api.pavex.dev` or `www.pavex.dev`.

## Domain parameters

If your needs are more complex, you can make your domain guards dynamic:

--8<-- "doc_examples/guide/routing/domain_guards/project-dynamic.snap"

`{sub}` is a **domain parameter**.\
It matches everything before `.pavex.dev`, up to the previous `.` or the beginning of the domain.\
It matches, for example, `api.pavex.dev` and `ui.pavex.dev`. It won't match `admin.api.pavex.dev` or `pavex.dev` though!

You can have multiple domain parameters in the same domain guard, as long as they are separated by a `.`:

--8<-- "doc_examples/guide/routing/domain_guards/project-multi.snap"

### Catch-all parameters

Normal domain parameters are limited to a single DNS label—i.e. they stop at the previous `.` or at the end of the domain.
You can use the `*` character to craft a **catch-all domain parameter**. It matches the rest of the domain, regardless of its contents:

--8<-- "doc_examples/guide/routing/domain_guards/project-catch_all.snap"

`{*any}` matches everything **before** `example.dev`, even if it contains `.` separators.\
`{*any}.example.dev` matches, for example, `api.example.dev` and `ui.example.dev`, but it also matches `admin.api.example.dev`.

To avoid ambiguity,
you can have **at most one catch-all parameter in a domain guard** and it must be located **at the very beginning of the domain**.

## Domain detection

The domain requested by the client is determined using [the `Host` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Host).
If the header is not present, none of your domain guards will match.

## Security

[The `Host` header can be easily spoofed by the client](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/07-Input_Validation_Testing/17-Testing_for_Host_Header_Injection).
You shouldn't rely on domain guards for auth or other security-sensitive checks.

## Restrictions

Domain guards are an all-or-nothing deal.\
**Either you specify a domain guard for all routes in a blueprint, or you don't specify any at all.**

We recommend specifying domain guards at the very top, clearly partitioning your routes
according to the domain they should be served on, as shown in the very first example for this guide.

The only exception to this rule are fallbacks: you can register a top-level fallback that will be invoked
when no domain guard matches.

--8<-- "doc_examples/guide/routing/domain_guards/project-fallback.snap"

## Absolute form

Pavex doesn't make a distinction between absolute and relative domain names.\
If there a single trailing `.` at the end of a domain name, it will be stripped. For example,
Pavex treats `pavex.dev` and `pavex.dev.` as the same domain.
