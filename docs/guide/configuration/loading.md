# Loading

Pavex configuration system is **hierarchical** and **profile-based**.\
Pavex provides a utility, [`ConfigLoader`][ConfigLoader], to load
configuration when the application starts.

## Configuration profile

Different runtime envinroments require different configuration values—e.g. you
don't want to connect to your production database when running the application on
your development machine.

In a Pavex application, each runtime environment is modeled as a **configuration profile**.
The profile is loaded from the `PX_PROFILE` environment variable, [unless explicitly specified](/api_reference/pavex/config/struct.ConfigLoader.html#method.profile).

Configuration profiles are conventionally defined in the [`server` crate](/guide/project_structure/server.md).
They must implement the [`ConfigProfile`][ConfigProfileT] trait, and they
are often modelled as enums:

--8<-- "docs/examples/configuration/profile.snap"

1. The [`ConfigProfile`][ConfigProfileT] trait can be derived using the [`ConfigProfile`][ConfigProfileD] derive macro.
   By default, the name of the profile is the snake_case representation of the enum variant name, but
   it can be customized via the `#[px(profile = "...")]` helper attribute.

### Customize your profiles

Profiles should be specific to your application's needs.\
The project scaffolded by `pavex new` comes with two configuration profiles: `dev` and `prod`.
Feel free to extend this list as needed—e.g. a `staging` profile for your staging environment.

## Configuration sources

Pavex combines multiple configuration sources to assemble the configuration values
that are ultimately used to drive the application.

For a given profile, Pavex combines the following sources:

1. Environment variables (`PX_*`)
2. Profile-specific configuration file (`{configuration_dir}/{profile}.yml`)
3. Base configuration file (`{configuration_dir}/base.yml`)

The list above is ordered by precedence: environment variables take precedence
over profile-specific configuration files, which in turn take precedence
over the base configuration file.

## Environment variables

Only environment variables prefixed with `PX_` are considered when loading configuration values.\
This is meant to reduce the risk of misconfiguration—e.g. loading a value from an unrelated
environment variable that's been set on the underlying system.

The naming convention for environment variables is `PX_{CONFIGURATION_KEY}__{FIELD_NAME}`, where:

- `{CONFIGURATION_KEY}` is [the key][config_key] you specified via [`#[pavex::config]`][config_attr].
- `{FIELD_NAME}` is the name of the field you are trying to set within that configuration entry.

As an example, consider this configuration type:

--8<-- "docs/examples/configuration/server_config.snap"

You would have to set `PX_SERVER__PORT` to configure its `port` field via environment variables.

### Nested fields

The naming convention supports nested fields too: add a double underscore (`__`) as separator
every time you access a nested field.

As an example, consider this configuration type:

--8<-- "docs/examples/configuration/postgres_config.snap"

You would have to set `PX_POSTGRES__POOL__MAX_SIZE` to configure the `max_size` field via environment variables.

## Configuration files

Configuration files are expected to be in the [YAML](https://yaml.org/) format.\
The default configuration directory is named `configuration`. It is specified as a **relative path**.
Pavex will start looking for it as a subdirectory of the current working directory;
if it doesn't exist, it will look in the parent directory, and so on, recursively, until it reaches the root directory.
It stops on the first matching directory.

You can also [customize the configuration directory path](/api_reference/pavex/config/struct.ConfigLoader.html#method.configuration_dir)
if needed.

[ConfigLoader]: /api_reference/pavex/config/struct.ConfigLoader.html
[ConfigProfileT]: /api_reference/pavex/config/trait.ConfigProfile.html
[ConfigProfileD]: /api_reference/pavex/config/derive.ConfigProfile.html
[config_key]: /guide/configuration/entries.md#configuration-key
[config_attr]: /api_reference/pavex/attr.config.html
