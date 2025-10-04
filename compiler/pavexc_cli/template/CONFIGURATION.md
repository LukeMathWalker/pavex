# Configuration

The source of truth for configuration is the `Config` type in `server/src/configuration.rs`.\
Check out Pavex's [configuration documentation](https://pavex.dev/docs/guide/configuration)
for an introduction to Pavex's configuration system.

## Profile

This application can be run using one out of two different profiles: `dev` and `prod`.\
You can specify the app profile that you want to use by setting the `PX_PROFILE` environment variable; e.g.:

```bash
PX_PROFILE=prod cargo px run
```

for running the application with the `prod` profile.

By default, `cargo run` uses `dev` profile.\
This happens because `PX_PROFILE` is set to `dev` in the `[env]` section of [.cargo/config.toml](.cargo/config.toml).\
You can use the `[env]` section to specify additional environment variables that you want to set when running the application locally.

## Sensitive local env variables

Be careful, though! `.cargo/config.toml` is committed to version control, so you should avoid storing sensitive information there.\
If you need local environment variables that shouldn't be shared with other developers, you can use a `.env` file at the root of the repository. For example:

```bash
# .env
# This file is used to store sensitive local environment variables
PX_APP__DATABASE__PASSWORD=super_secret_password
```

The application will automatically load values from the `.env` file, if one exists, and `.gitignore` is already set up to ignore it, thus
preventing secret information from being committed to version control.
