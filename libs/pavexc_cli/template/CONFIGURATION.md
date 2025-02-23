# Configuration

All configurable parameters are listed in `server/src/configuration/schema.rs`.

Configuration values are loaded from two sources:

- Configuration files
- Environment variables

Environment variables take precedence over configuration files.

## Profile

The application can be run using one out of two different profiles: `dev` and `prod`.\
You can specify the app profile that you want to use by setting the `APP_PROFILE` environment variable; e.g.:

```bash
APP_PROFILE=prod cargo px run
```

for running the application with the `prod` profile.

## Configuration files

All configuration files are in the `server/configuration` folder.\
The settings that you want to share across all profiles should be placed
in `server/configuration/base.yml`.
Profile-specific configuration files can be then used
to override or supply additional values on top of the default settings (
e.g. `server/configuration/dev.yml`).

## Local env variables

By default, `cargo run` uses `dev` profile.\
This happens because `APP_PROFILE` is set to `dev` in the `[env]` section of [.cargo/config.toml](.cargo/config.toml).\
You can use the `[env]` section to specify additional environment variables that you want to set when running the application locally.

## Sensitive local env variables

Be careful, though! `.cargo/config.toml` is committed to version control, so you should avoid storing sensitive information there.\
If you need local environment variables that shouldn't be shared with other developers, you can use a `.env` file at the root of the repository. For example:

```bash
# .env
# This file is used to store sensitive local environment variables
DATABASE_PASSWORD=super_secret_password
```

The application will automatically load values from the `.env` file, if one exists, and `.gitignore` is already set up to ignore it, thus
preventing secret information from being committed to version control.
