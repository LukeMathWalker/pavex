# Documentation

This folder contains the documentation that's published to [pavex.dev](https://pavex.dev).\
The auto-generated documentation for the Rust crates serves as an API reference; this documentation, instead,
serves as a higher-level guide to the project: quickstart, tutorials, recipes, concept deep-dives, etc.

It's built using [MkDocs](https://www.mkdocs.org/)
and [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/).

## Prerequisites

We don't want to force you to set up a Python dev environment: we rely on Docker to build and preview the docs.
Make sure you have [Docker](https://www.docker.com/) installed and running.

You'll then need to build the relevant image:

```bash
docker build -t pavex-docs .
```

## Commands

You can preview the docs locally by running from the root of the repository (i.e., the parent folder of
the directory containing this README file):

```bash
docker run --rm -it -p 8001:8000 \
  -v ${PWD}:/docs \
  pavex-docs
```

The docs will be available at [http://localhost:8001](http://localhost:8001) and will auto-reload when you make changes.

The docs embed the auto-generated API reference for the first-party Pavex crates: the command above mounts the
relevant folders so that the docs can access the generated HTML files, but it **won't (re)generate them for you**.\
If you want to generate or update the API reference,
you'll need to run the following command from the root of the repository:

```bash
pushd libs && cargo api_ref && cp -r target/doc/* ../docs/api_reference
```
