# Documentation

This folder contains the documentation that's published to [pavex.dev](https://pavex.dev).\
The auto-generated documentation for the Rust crates serves as an API reference; this documentation, instead,
serves as a higher-level guide to the project: quickstart, tutorials, recipes, concept deep-dives, etc.

It's built using [MkDocs](https://www.mkdocs.org/)
and [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/).

## Prerequisites

Install [`uv`](https://docs.astral.sh/uv/getting-started/installation/), a Python toolchain manager and built tool.
Then run:

```bash
uv sync
```

from the root of the repository.

## Commands

You can preview the docs locally by running from the root of the repository (i.e., the parent folder of
the directory containing this README file):

```bash
uv run mkdocs serve --open
```

The docs will be available at [http://localhost:8000](http://localhost:8000) and will auto-reload when you make changes.

The docs embed the auto-generated API reference for the first-party Pavex crates: the command above mounts the
relevant folders so that the docs can access the generated HTML files, but it **won't (re)generate them for you**.\
If you want to generate or update the API reference,
you'll need to run the following commands from the root of the repository:

```bash
mkdir -p docs/api_reference
cargo api_ref && cp -r target/doc/* docs/api_reference
```
