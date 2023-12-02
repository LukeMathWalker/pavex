---
title: Why Pavex?
---
## Batteries included

Pavex aims to be a **one-stop shop** for your API projects,
providing all the batteries you need to build something production-ready.  
You can expect **first-class solutions for common API development tasks** (e.g. authentication, background jobs, telemetry, etc.)
or an **opinionated recommendation** for specific high-quality third-party libraries (e.g. async executors, cryptography, etc.), with a smooth integration into the framework.

!!! note

    The first beta release of the framework will focus on the foundations: manipulating HTTP requests and responses, routing, middleware, dependency injection, error handling. We'll then iterate on the framework to add more batteries as the core layer stabilizes.

## Productive

Rust is a great language, but it has a large surface area: it's easy to feel like you never know enough to actually get started.
Pavex aims to **lower the barrier of entry**: you'll only need **an understanding of Rust core concepts to get up and running**.
When things go wrong, we'll be there to help you: Pavex's transpiler will provide **detailed error messages** to help you understand what went wrong and how to fix it.

```
ERROR:
× I can't invoke your wrapping middleware, `timeout`, because it needs an instance of
│ `TimeoutConfig` as input, but I can't find a constructor for that type.
│
│     ╭─[src/blueprint.rs:18:1]
│  18 │
│  19 │     bp.wrap(f!(crate::timeout));
│     ·             ────────┬────────
│     ·                     ╰── The wrapping middleware was registered here
│  20 │
│     ╰────
│    ╭─[src/load_shedding.rs:5:1]
│  5 │
│  6 │ pub async fn timeout<T>(next: Next<T>, timeout_config: TimeoutConfig) -> Response
│    ·                                                        ──────┬──────
│    ·                I don't know how to construct an instance of this input parameter
│    ╰────
│   help: Register a constructor for `TimeoutConfig`
```

## Safe

You can't be productive if you spend most of your time hunting down bugs.
Pavex's transpiler will **catch as many errors as possible at compile-time**, providing you with actionable feedback to fix them. This static analysis is on top of the usual Rust compiler checks: Pavex performs additional domain-specific checks at compile-time to ensure that your code behaves as expected at runtime.

This is all embedded into the framework. You don't have to install any additional tools or plugins, and it doesn't impact the complexity of the framework's API: we try to keep type complexity to a minimum, without sacrificing compile-time safety.

## Flexible

Pavex is an **opinionated framework**, but it's designed to be **flexible**.
You can swap out any component of the framework with your own implementation, or defer to a third-party library.

It's a pragmatic choice: we expect most projects to align with the vast majority of Pavex's design decisions, but each environment brings its own requirements (especially in enterprise) and we want to make sure that you can adapt the framework to your needs.
