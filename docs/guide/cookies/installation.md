# Installation

The cookie machinery is not included in the project scaffolded by `pavex new`.
You need to add a few lines to set it up:

--8<-- "docs/examples/cookies/installation.snap"

1. Bring `pavex`'s cookie components into scope (e.g. [`ResponseCookies`][ResponseCookies]).
2. Attach cookies to the outgoing response.

It's enough to add [`INJECT_RESPONSE_COOKIES`][INJECT_RESPONSE_COOKIES] to your middleware stack
if you're already importing components from the `pavex` crate.

[Blueprint]: /api_reference/pavex/struct.Blueprint.html
[CookieKit]: /api_reference/pavex/cookie/struct.CookieKit.html
[ProcessorConfig]: /api_reference/pavex/cookie/struct.ProcessorConfig.html
[ProcessorConfig::default]: /api_reference/pavex/cookie/struct.ProcessorConfig.html#method.default
[ProcessorConfig::crypto_rules]: /api_reference/pavex/cookie/struct.ProcessorConfig.html#structfield.crypto_rules
[default settings]: /api_reference/pavex/cookie/struct.ProcessorConfig.html#fields
[ResponseCookies]: /api_reference/pavex/cookie/struct.ResponseCookies.html
[INJECT_RESPONSE_COOKIES]: /api_reference/pavex/cookie/fn.inject_response_cookies.html
