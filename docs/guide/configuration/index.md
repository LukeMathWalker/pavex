# Configuration

Configuration changes the behavior of your application without
having to modify its source code.
Configuration is a good fit for:

- Values that change based on the environment the application is running in.
- Secrets that you want to keep out of version control.
- Parameters that you may want to tweak at runtime without having to redeploy.

API keys, database credentials, feature flags, logging level—these are all commonly
managed via configuration.

## In this guide

Configuration is a first-class concept in Pavex and this guide provides an in-depth
introduction to Pavex configuration system.

["New entries"](entries.md) and ["Loading"](loading.md) are must-reads: they explain
how to add configuration options to your application and how Pavex loads configuration values.
