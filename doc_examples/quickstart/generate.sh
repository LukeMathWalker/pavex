#!/usr/bin/env bash
set -euo pipefail

pavex new demo
# Generate the code for the SDK crate
cd demo && cargo px c
