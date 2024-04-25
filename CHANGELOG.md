# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
