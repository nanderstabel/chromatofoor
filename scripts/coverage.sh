#!/usr/bin/env bash
set -euo pipefail

if ! cargo llvm-cov --version >/dev/null 2>&1; then
    echo "Error: 'cargo-llvm-cov' is not installed."
    echo "Install it with:"
    echo "  rustup component add llvm-tools-preview clippy rustfmt --toolchain nightly"
    echo "  cargo install cargo-llvm-cov --locked"
    exit 1
fi

MODE="${1:---summary}"
IGNORE_REGEX='(^|/)(tests|testbed)/'

case "$MODE" in
    --summary)
        cargo +nightly llvm-cov \
            --workspace \
            --all-features \
            --doctests \
            --ignore-filename-regex "$IGNORE_REGEX"
        ;;
    --html)
        cargo +nightly llvm-cov \
            --workspace \
            --all-features \
            --doctests \
            --ignore-filename-regex "$IGNORE_REGEX" \
            --html \
            --open
        ;;
    --codecov)
        cargo +nightly llvm-cov \
            --workspace \
            --all-features \
            --doctests \
            --ignore-filename-regex "$IGNORE_REGEX" \
            --codecov \
            --output-path codecov.json
        echo "Wrote Codecov report to codecov.json"
        ;;
    --lcov)
        cargo +nightly llvm-cov \
            --workspace \
            --all-features \
            --doctests \
            --ignore-filename-regex "$IGNORE_REGEX" \
            --lcov \
            --output-path lcov.info
        echo "Wrote LCOV report to lcov.info"
        ;;
    *)
        echo "Usage: $0 [--summary | --html | --codecov | --lcov]"
        exit 1
        ;;
esac
