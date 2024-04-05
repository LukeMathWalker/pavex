# Cookies

[HTTP cookies](https://developer.mozilla.org/en-US/docs/Web/HTTP/Cookies) can be used to keep state across multiple HTTP requests.  
They are often used by web applications that interact with browsers to manage authenticated sessions, shopping cart contents, 
and other kinds of ephemeral user-specific data.

## Installation

The machinery required to manipulate cookies is not included by default in the project scaffolded by `pavex new`.
You need to go through a few steps to set it up.

### `CookieKit`

[`CookieKit`][CookieKit] bundles together all the components you need to manipulate cookies. 
Register it with your [`Blueprint`][Blueprint] to get started:

// Code example

You can customize each component inside [`CookieKit`][CookieKit] to suit your needs. Check out the [Kits](dependency_injection/core_concepts/kits.md) 
section for reference examples.

### `ProcessorConfig`

[`ProcessorConfig`][ProcessorConfig] determines how cookies (both incoming and outgoing) are processed by your application.
Do they need to be percent-encoded? Should they be signed or encrypted? Using what key?

Once you register [`CookieKit`][CookieKit] against your [`Blueprint`][Blueprint], you'll be asked to provide an instance of 
[`ProcessorConfig`][ProcessorConfig] as an input to your [`build_application_state`][build_application_state] function.  

You have two options:

1. Add [`ProcessorConfig`][ProcessorConfig] as a field on your application's `AppConfig` struct, usually located
   in `app/src/configuration.rs`.
   If you annotate it with `#[serde(default)]`, it'll fall back on the [default settings] unless you override them in your
   configuration files or with environment variables.
2. Manually build an instance of [`ProcessorConfig`][ProcessorConfig] in the `main` function of your `server` crate and 
   then pass it to [`build_application_state`][build_application_state].
   You can use [`ProcessorConfig::default`][ProcessorConfig::default] if you're happy with the [default settings].

We recommend the first approach.


[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[CookieKit]: ../../api_reference/pavex/cookie/struct.CookieKit.html
[ProcessorConfig]: ../../api_reference/pavex/cookie/struct.ProcessorConfig.html
[build_application_state]: ../../api_reference/pavex/server/fn.build_application_state.html
[ProcessorConfig::default]: ../../api_reference/pavex/cookie/struct.ProcessorConfig.html#method.default
[default settings]: ../../api_reference/pavex/cookie/struct.ProcessorConfig.html#fields
[build_application_state]: ../../project_structure.md#applicationstate
