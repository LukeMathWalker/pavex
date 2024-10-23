# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.49](https://github.com/LukeMathWalker/pavex/compare/0.1.48...0.1.49) - 2024-10-23

### Added

- First release of Pavex's HTTP sessions toolkit 🎉 ([#338](https://github.com/LukeMathWalker/pavex/pull/338))
- Distinguish between methods and functions in error messages ([#344](https://github.com/LukeMathWalker/pavex/pull/344))
- Start caching the JSON documentation of path dependencies. We rely on the hash of their contents to avoid serving stale data
- Pavex will no longer emit duplicated diagnostics, thus reducing visual noise when code generation fails

### Fixed

- Pavex always uses a public path to refer to public items, even if they are defined in a private module
- Detect infinite paths and break early to avoid stalls when generating server SDK crates
- Ensure error observers are correctly added when dealing with errors in the call graph of a middleware of any kind
- Perform cross-call-graph analysis to determine if additional `.clone()` statements are needed before invoking a middleware.
- Don't discard spans if they match the provided log filter in pavexc

### Other

- Update dependencies to latest possible version. In particular, update 'rustdoc-types' and the nightly version used by 'pavexc'

## [0.1.48](https://github.com/LukeMathWalker/pavex/compare/0.1.47...0.1.48) - 2024-09-02

### Fixed
- Avoid panic petgraph-related panic when inserting clone nodes to fix borrow checking errors ([#334](https://github.com/LukeMathWalker/pavex/pull/334))

## [0.1.47](https://github.com/LukeMathWalker/pavex/compare/0.1.46...0.1.47) - 2024-08-14

### Fixed
- always use the specified toolchain, remove yet another location where nightly was hard-coded

### Other
- Pin a specific `nightly` version for each version of `pavexc`, ensuring they are compatible ([#331](https://github.com/LukeMathWalker/pavex/pull/331))
- Allow overriding the `nightly` toolchain used to generate JSON docs via `PAVEXC_DOCS_TOOLCHAIN` ([#331](https://github.com/LukeMathWalker/pavex/pull/331))
- Fix panics when performing dependency injection for complex call graphs ([#329](https://github.com/LukeMathWalker/pavex/pull/329))

## [0.1.46](https://github.com/LukeMathWalker/pavex/compare/0.1.45...0.1.46) - 2024-07-27

### Other
- Feature gate tokio net feature for pavex behind server feature ([#324](https://github.com/LukeMathWalker/pavex/pull/324))
- update Cargo.toml dependencies

## [0.1.45](https://github.com/LukeMathWalker/pavex/compare/0.1.44...0.1.45) - 2024-07-02

### Added
- enable 'std' feature on the 'time' crate in 'pavex'

### Fixed
- std's collections can be used as prebuilt types ([#321](https://github.com/LukeMathWalker/pavex/pull/321))

### Other
- Add constructor for RequestHead ([#319](https://github.com/LukeMathWalker/pavex/pull/319))

## [0.1.44](https://github.com/LukeMathWalker/pavex/compare/0.1.43...0.1.44) - 2024-06-22

### Fixed
- Don't use public items via paths that include private modules ([#316](https://github.com/LukeMathWalker/pavex/pull/316))

## [0.1.43](https://github.com/LukeMathWalker/pavex/compare/0.1.42...0.1.43) - 2024-06-19

### Added
- Add status_mut() function to Response ([#313](https://github.com/LukeMathWalker/pavex/pull/313))

## [0.1.42](https://github.com/LukeMathWalker/pavex/compare/0.1.41...0.1.42) - 2024-06-18

### Fixed
- elided lifetime parameters in generic structs are handled correctly ([#310](https://github.com/LukeMathWalker/pavex/pull/310))

## [0.1.41](https://github.com/LukeMathWalker/pavex/compare/0.1.40...0.1.41) - 2024-06-16

### Fixed
- Pavex will reject singleton constructors if they return a type with non-`'static` lifetime parameters. Singletons must be shared across worker threads, therefore they must be `'static`.

## [0.1.40](https://github.com/LukeMathWalker/pavex/compare/0.1.39...0.1.40) - 2024-06-16

### Fixed
- 'pavex new' no longer panics if 'cargo fmt' fails.  ([#303](https://github.com/LukeMathWalker/pavex/pull/303))

## [0.1.39](https://github.com/LukeMathWalker/pavex/compare/0.1.38...0.1.39) - 2024-06-15

### Added
- Add a workspace-hack crate to the generated starter project to minimise (re)build times
- Introduce prebuilt types ([#298](https://github.com/LukeMathWalker/pavex/pull/298))
- Add a new '--template' option to 'pavex new' and 'pavexc new'. It includes a dedicated 'quickstart' template as well as the 'api' template, the default.
- In the starter project, use a meaningful example to showcase how the configuration system works, rather than a dummy with no usage
- Shorthand methods (`.clone_if_necessary()` and `.never_clone()`) to tweak the default cloning strategy on constructors and prebuilt types

### Fixed
- Set new Cargo lint to allow 'cfg(pavex_ide_hint)' in Pavex, its snapshot tests and its scaffolded projects
- Use the [env] section of .cargo/config.toml to store non-sensitive env variables used for local development. It fixes configuration for newly generated projects.
- Don't use colored logs if color is not enabled.
- Include `super` and `self` as valid prefixes for relative paths in error messages ([#296](https://github.com/LukeMathWalker/pavex/pull/296))
- You can no longer register a type with a non-`'static` lifetime parameter (implicit or explicit) as a singleton. ([#298](https://github.com/LukeMathWalker/pavex/pull/298))

## [0.1.38](https://github.com/LukeMathWalker/pavex/compare/0.1.37...0.1.38) - 2024-04-28

### Added
- Rework CLI introspections ([#292](https://github.com/LukeMathWalker/pavex/pull/292))

### Fixed
- anyhow::Result<Self> can be returned from constructors and other fallible components ([#293](https://github.com/LukeMathWalker/pavex/pull/293))

## [0.1.37](https://github.com/LukeMathWalker/pavex/compare/0.1.36...0.1.37) - 2024-04-27

### Fixed
- Interpolate error message when failing to download a prebuilt `pavexc` binary
- Remove dependency on OpenSSL on Linux

## [0.1.36](https://github.com/LukeMathWalker/pavex/compare/0.1.35...0.1.36) - 2024-04-27

### Fixed
- Use the correct name for package names that contain hyphens in the (generated) server SDK Cargo.toml ([#287](https://github.com/LukeMathWalker/pavex/pull/287))
  For example, `sqlx-query` used to be renamed to `sqlx_query` in the generated `Cargo.toml`, causing a `cargo` error.

### Other
- Update dependencies ([#285](https://github.com/LukeMathWalker/pavex/pull/285))
- Activation keys are now validated server-side ([#283](https://github.com/LukeMathWalker/pavex/pull/283))

## [0.1.35](https://github.com/LukeMathWalker/pavex/compare/0.1.34...0.1.35) - 2024-04-26

### Fixed
- Allow &mut references to be held by Next's state. ([#280](https://github.com/LukeMathWalker/pavex/pull/280))

## [0.1.34](https://github.com/LukeMathWalker/pavex/compare/0.1.33...0.1.34) - 2024-04-25

### Fixes
- Use Unix path separator in Cargo.toml manifests when specifying path dependencies ([#275](https://github.com/LukeMathWalker/pavex/pull/275))
  It allows the same Pavex project to be built on all platforms with no changes.
- Re-add default .env file ([#276](https://github.com/LukeMathWalker/pavex/pull/276))
  It allows `cargo px r` to "just work" on a newly scaffolded Pavex project.

### Other
- Re-order Cargo.toml file ([#277](https://github.com/LukeMathWalker/pavex/pull/277))

## [0.1.33](https://github.com/LukeMathWalker/pavex/compare/0.1.32...0.1.33) - 2024-04-21

### Added
- Server request id is now represented as a TypeId ([#272](https://github.com/LukeMathWalker/pavex/pull/272))

## [0.1.31](https://github.com/LukeMathWalker/pavex/compare/0.1.30...0.1.31) - 2024-04-21

### Other
- Centralize version.
