#!/bin/sh
if [ "$RUSTFMT" = "true" ]; then
    cargo fmt --all -- --write-mode diff
else
    cargo build -vv --features "$FEATURES"
    cargo doc --no-deps --features "$FEATURES"
fi
