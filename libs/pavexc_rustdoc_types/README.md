# `pavexc_rustdoc_types`

It's a fork of [`rustdoc_types`](https://github.com/rust-lang/rustdoc-types) with a specific purpose: maximise performance
when working with JSON docs in `pavexc`.\
In particular, we want to minimise the size of the JSON files stored in `pavexc` SQLite database as well as the time it
takes to parse the JSON files returned by `rustdoc`.

## Changes

The changes are rather minimal: we comment out some fields that are not used in `pavexc` but account for a significant
portion of the JSON file size. They are all marked out with a `// EXCLUDED!` comment.

We rely on this script:

```bash
total_size=$(wc -c < file.json); docs_size=$(jq '.. | .field_name1? // .field_name2? // empty' file.json | wc -c); echo "scale=2; $docs
_size*100/$total_size" | bc
```

to measure the impact of removing `field_name1` and `field_name2` from the JSON file. The script calculates the percentage
of the JSON file size that is taken up by `field_name1` and `field_name2`.

We strive to remove fields that account for a non-negligible portion of the JSON file size, to minimise the overhead of syncing
the fork with the upstream repository.
