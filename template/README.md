# Pavex starter template

This is the official starter template for a new [Pavex](https://pavex.dev) project.  

Run

```bash
pavex new my-api
```

to create a new project based on this template.

## Overview

The template is built on top of [`cargo-generate`](https://github.com/cargo-generate/cargo-generate).

The generated project includes:

- An API with a single endpoint (`GET /api/status`) that returns a `200 OK` response
- An integration test which verifies that the endpoint works as expected
- The scaffolding required to generate the server SDK code based on the application blueprint