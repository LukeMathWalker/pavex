# 2022-08-24

## Issues

- Re-exports are a mess. We need to work on the discrepancy between the first segment of the path and the name of the crate referenced by the `id`. E.g.

```text
ResolvedPath(
    Path {
        name: "pavex_runtime::hyper::body::Body",
        id: Id(
            "35:485:1603",
        ),
        args: Some(
            AngleBracketed {
                args: [],
                bindings: [],
            },
        ),
    },
),
```

# 2022-08-10

## Issues

- What item kinds are available in the `paths` section of rustdoc's JSON output? It turns out, not all of them (see `cat <filepath> | jq "[.paths[].kind] | unique"`). In particular, not methods!

# 2022-07-27

## Issues

- We need to enforce that components do not depend on other components with a shorter lifecycle - e.g. singletons requiring request-scoped or transient types to be built. It might make sense to allow some transient-like types that are scoped to the initialization phase of the server, but it probably won't be necessary for the MVP.

# 2022-07-19

## Open questions

- Is there an easy way to render DOT graph files to ASCII art without having to pull in Perl (graph-easy)?

# 2022-07-06

## Open questions

- How do we generate type information for `std`/`alloc`/`core` via `rustdoc`? `rustdoc` doesn't seem to treat them as "normal" dependencies, therefore the obvious command (the one we are using for all other crates) fails.

# 2022-06-21

## Findings

- [`cargo-public-api`](https://github.com/Enselic/cargo-public-api) is making extensive usage of `rustdoc`'s JSON output. We can take inspiration/use it as an example.

# 2022-06-20

## Open questions

- Is it possible to write a macro that rejects a function that is not specified using its fully qualified name? E.g. `syn::parse` is good, `parse` is bad.
- If we are constraining handler and middleware functions to be passed using fully qualified paths, how do we plan to handle crate renames/aliases specified in their `Cargo.toml`? Can we somehow embed into the `Blueprint` the name of the crate that attached the handler/middleware function, in order to then resolve the first segment of the fully qualified path "in context"?
  - This is possible by using `CARGO_CRATE_NAME`/`CARGO_PKG_NAME`. See this [list of env variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates) passed by `cargo` when compiling a project. We are adding a dependency on `cargo` here, which could become an obstacle for orgs using custom build systems - this is acceptable for now.

## Findings

- It looks like `rust-analyzer` can't serialize its state to disk in order to perform some kind of incremental compilation when regenerating a `Blueprint` over and over again in the developer inner loop. See [this issue](https://github.com/salsa-rs/salsa/issues/10)
- Using fully qualified paths for handler and middleware functions seems the only reasonable path forward if we expect a `BlueprintBuilder` to contain all the information required to generate a `Blueprint`. If relative import paths are used, we have no way to determine what they refer to when we try to build the `Blueprint`. There _might_ be a way to get around this limitation, but it feels unnecessary complex at this stage.