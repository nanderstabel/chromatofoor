#!/usr/bin/env bash

set -euo pipefail

echo "Building graph with graphify-rs..."
# We need to capture the output to parse the directory, but we also want the user to see it.
# We'll use a temporary file to capture the output since 'tee /dev/tty' can be tricky in headless environments.
TMP_OUTPUT=$(mktemp)
graphify-rs build 2>&1 | tee "$TMP_OUTPUT"

# Extract the output directory
OUT_DIR=$(grep -oP '✓ Done! Output in \K.*' "$TMP_OUTPUT" | tail -n 1)
rm "$TMP_OUTPUT"

if [ -z "$OUT_DIR" ]; then
    echo "Error: Could not determine graphify-rs output directory."
    exit 1
fi

REPORT_FILE="$OUT_DIR/GRAPH_REPORT.md"

if [ ! -f "$REPORT_FILE" ]; then
    echo "Error: $REPORT_FILE not found."
    exit 1
fi

echo "Injecting OKF frontmatter into $REPORT_FILE..."

FRONTMATTER="---
title: Codebase Structural Graph
type: generated_report
auto_generated: true
description: High-level codebase summary, God Nodes, and logical communities.
---"

# Prepend frontmatter
TEMP_FILE=$(mktemp)
echo "$FRONTMATTER" > "$TEMP_FILE"
cat "$REPORT_FILE" >> "$TEMP_FILE"
mv "$TEMP_FILE" "$REPORT_FILE"

echo "Successfully injected OKF frontmatter."
