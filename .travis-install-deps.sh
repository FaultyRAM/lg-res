#!/bin/sh
if [ "$RUSTFMT" = "true" ]; then
    rustup -v component add rustfmt-preview
fi
