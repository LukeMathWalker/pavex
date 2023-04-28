#!/bin/bash

# Check that the first argument is not empty
if [ -z "$1" ]; then
    echo "Usage: ./ci.sh <cargo command> [<cargo options>]"
    echo "Runs a 'cargo' command on all Rust workspaces in the current directory and its subdirectories."
    exit 1
fi

# Get the cargo command and options from the arguments
CARGO_CMD=$1
shift
CARGO_OPTS=${*:-""}

# Find all directories that contain a Cargo.toml file and have a [workspace] section in the file
WORKSPACES=$(find . -type f -name Cargo.toml -exec grep -q "\[workspace\]" {} \; -print | xargs -n1 dirname | sort | uniq)

# Exclude test directory 
TOP_WORKSPACES=()
for workspace in $WORKSPACES; do
    if [[ ! "$workspace" =~ "/ui_test_envs/" ]]; then
        TOP_WORKSPACES+=("$workspace")
    fi
done

# Iterate over each workspace and run `cargo check`
for workspace in "${TOP_WORKSPACES[@]}"; do
    echo "Running 'cargo $CARGO_CMD $CARGO_OPTS' in workspace: $workspace"
    (cd "$workspace" && cargo $CARGO_CMD $CARGO_OPTS)
done