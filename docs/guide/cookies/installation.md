# Installation

The machinery required to manipulate cookies is not included by default in the project scaffolded by `pavex new`.
You need to go through a few steps to set it up.

## `CookieKit`

[`CookieKit`][CookieKit] bundles together all the components you need to manipulate cookies.
Register it with your [`Blueprint`][Blueprint] to get started:

--8<-- "doc_examples/guide/cookies/installation/project-kit.snap"

You can customize each component inside [`CookieKit`][CookieKit] to suit your needs.
Check out the ["Kits"](../dependency_injection/kits.md#customization)
section for reference examples.

## `ProcessorConfig`

[`ProcessorConfig`][ProcessorConfig] determines how cookies (both incoming and outgoing) are processed by your application.
Do they need to be percent-encoded? Should they be signed or encrypted? Using what key?

Once you register [`CookieKit`][CookieKit] against your [`Blueprint`][Blueprint], you'll get an error:

```ansi-color
--8<-- "doc_examples/guide/cookies/installation/missing_process_config.snap"
```

To fix it, you need to give Pavex a way to work with a [`ProcessorConfig`][ProcessorConfig] instance.
You have two options:

1. Add [`ProcessorConfig`][ProcessorConfig] as a field on your application's `AppConfig` struct, usually located
   in `app/src/configuration.rs`.
   Check out [the "Realworld" example](https://github.com/LukeMathWalker/pavex/blob/883aed7b8c85bd97e0df5edda12025dd3a51f9b9/examples/realworld/app/src/configuration.rs#L16)
   as a reference (1).
   { .annotate }

       1. If you annotate the field with `#[serde(default)]`, it'll fall back on the [default settings] unless you override them in your
          configuration files or with environment variables.

2. Use [`ProcessorConfig::default`][ProcessorConfig::default] as its constructor if you're happy with the [default settings].
   --8<-- "doc_examples/guide/cookies/installation_with_default/project-default.snap"

The second option is the quickest way to get started.\
You'll have to switch to the first approach if you need to customize [`ProcessorConfig`][ProcessorConfig].
That'll be necessary, for example, if you rely on cookies for sensitive information, such as session tokens.
You'll have to configure [`ProcessorConfig::crypto_rules`][ProcessorConfig::crypto_rules] to ensure those cookies are
encrypted or signed.

[Blueprint]: ../../api_reference/pavex/blueprint/struct.Blueprint.html
[CookieKit]: ../../api_reference/pavex/cookie/struct.CookieKit.html
[ProcessorConfig]: ../../api_reference/pavex/cookie/struct.ProcessorConfig.html
[ProcessorConfig::default]: ../../api_reference/pavex/cookie/struct.ProcessorConfig.html#method.default
[ProcessorConfig::crypto_rules]: ../../api_reference/pavex/cookie/struct.ProcessorConfig.html#structfield.crypto_rules
[default settings]: ../../api_reference/pavex/cookie/struct.ProcessorConfig.html#fields
[build_application_state]: ../project_structure.md#applicationstate
