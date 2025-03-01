# `ApplicationConfig`

Every [configuration entry](entries.md) becomes a field in `ApplicationConfig`,
a code-generated struct in [the server SDK crate](/guide/project_structure/server_sdk.md) that represents
the entire set of configuration options for your application.

--8<-- "doc_examples/guide/configuration/01-build_state.snap"

## Usage

`ApplicationConfig` is used as the generic parameter for [`ConfigLoader::load`][ConfigLoader::load]
when assembling the configuration as the application starts.

You can also use `ApplicationConfig` as the source of truth to determine the names of
[environment variables](loading.md#environment-variables) and
[the expected schema for configuration files](loading.md#configuration-files).

Other than that, you won't need to interact with `ApplicationConfig` directly
unless you're implementing a custom configuration loading mechanism.

[ConfigLoader::load]: /api_reference/pavex/config/struct.ConfigLoader.html#method.load
