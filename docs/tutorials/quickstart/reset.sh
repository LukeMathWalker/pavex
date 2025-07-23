#!/bin/bash
# Use the latest version of the quickstart template
# as the starting point for the quickstart tutorial
set -o pipefail

rm -rf project
PAVEXC_TEMPLATE_VERSION_REQ="0.1" PAVEX_PAVEXC=pavexc pavex new --template="quickstart" demo
mv demo project
rm project/Cargo.toml
rm -rf project/.git
