# Installation

The cookie machinery is not included in the project scaffolded by `pavex new`.
You need to go through a few steps to set it up.

## `CookieKit`

[`CookieKit`][CookieKit] bundles together all the components you need to manipulate cookies.
Register it with your [`Blueprint`][Blueprint] to get started:

--8<-- "doc_examples/guide/cookies/installation/project-kit.snap"

You can customize each component inside [`CookieKit`][CookieKit] to suit your needs.
Check out the ["Kits"](../dependency_injection/kits.md#customization)
section for reference examples.

[Blueprint]: /api_reference/pavex/blueprint/struct.Blueprint.html
[CookieKit]: /api_reference/pavex/cookie/struct.CookieKit.html
[ProcessorConfig]: /api_reference/pavex/cookie/struct.ProcessorConfig.html
[ProcessorConfig::default]: /api_reference/pavex/cookie/struct.ProcessorConfig.html#method.default
[ProcessorConfig::crypto_rules]: /api_reference/pavex/cookie/struct.ProcessorConfig.html#structfield.crypto_rules
[default settings]: /api_reference/pavex/cookie/struct.ProcessorConfig.html#fields
