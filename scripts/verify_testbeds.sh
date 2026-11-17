#!/bin/bash
set -e

RED_PHASE=false
if [ "${1:-}" = "--red-phase" ]; then
    RED_PHASE=true
elif [ -n "${1:-}" ]; then
    echo "Usage: $0 [--red-phase]"
    exit 1
fi

echo "=================================================="
if [ "$RED_PHASE" = true ]; then
    echo "    Verify Chromatofoor Testbeds (Red Phase)"
else
    echo "    Verify Chromatofoor Testbeds"
fi
echo "=================================================="
echo ""

# Ensure we are in the root directory
if [ ! -f "Cargo.toml" ] || [ ! -d "chromatofoor-gen" ]; then
    echo "Error: Please run this script from the root of the Chromatofoor repository."
    exit 1
fi

run_step() {
    local step_name="$1"
    shift

    if [ ! -t 1 ]; then
        echo "=== $step_name ==="
        "$@"
        return 0
    fi

    echo -ne "  ⏳ $step_name... \n"
    local tmp_log
    tmp_log=$(mktemp)

    "$@" > "$tmp_log" 2>&1 &
    local pid=$!

    local lines_to_show=5
    local printed_lines=0

    local cols
    cols=$(tput cols 2>/dev/null || echo 80)
    local max_len=$((cols - 12))
    if [ "$max_len" -lt 20 ]; then
        max_len=20
    fi

    while kill -0 "$pid" 2>/dev/null; do
        if [ "$printed_lines" -gt 0 ]; then
            for ((i=0; i<printed_lines; i++)); do
                echo -ne "\033[A\033[K"
            done
        fi

        local tail_output
        tail_output=$(tail -n "$lines_to_show" "$tmp_log" 2>/dev/null || true)

        printed_lines=0
        if [ -n "$tail_output" ]; then
            local lines=()
            IFS=$'\n' read -rd '' -a lines <<< "$tail_output" || true
            for line in "${lines[@]}"; do
                if [ "${#line}" -gt "$max_len" ]; then
                    line="${line:0:max_len}..."
                fi
                echo -e "\033[90m      | $line\033[0m"
                printed_lines=$((printed_lines + 1))
            done
        fi

        sleep 0.1
    done

    local exit_code=0
    wait "$pid" || exit_code=$?

    if [ "$printed_lines" -gt 0 ]; then
        for ((i=0; i<printed_lines; i++)); do
            echo -ne "\033[A\033[K"
        done
    fi

    echo -ne "\033[A\033[K"

    if [ "$exit_code" -eq 0 ]; then
        echo -e "  ✅ $step_name"
        rm -f "$tmp_log"
    else
        echo -e "  ❌ $step_name (Failed)"
        echo -e "\n--- Error Output ---"
        cat "$tmp_log"
        rm -f "$tmp_log"
        exit 1
    fi
}

build_generator() {
    cargo build -p chromatofoor-gen --all-features
}

lint_models() {
    if ! command -v esdm >/dev/null 2>&1; then
        echo "Note: external 'esdm' CLI not found on PATH; skipping external CLI check (covered by 'cargo test -p chromatofoor-gen')."
        return 0
    fi
    for testbed in testbed/*; do
        if [ -d "$testbed" ] && [ -f "$testbed/model.esdm.yaml" ]; then
            lint_output=$(esdm lint -d "$testbed" --format json 2>/dev/null || true)
            if [ -z "$lint_output" ] || ! echo "$lint_output" | jq -e . >/dev/null 2>&1; then
                esdm lint -d "$testbed"
                return 1
            fi
            violations=$(echo "$lint_output" | jq '[.[] | select(((.severity == "error") and (.ruleId != "esdm/structure/unknown-api-version" or (((.location.file | endswith(".infra.esdm.yaml")) or (.location.file | endswith(".lifecycle.esdm.yaml"))) | not))) or (.severity == "warning" and .ruleId != "esdm/modeling/event-name-with-aggregate-prefix"))]')
            count=$(echo "$violations" | jq 'length')
            if [ "$count" -gt 0 ]; then
                echo "ESDM linting failed for $testbed with strict rules:"
                echo "$violations" | jq -r '.[] | "\(.severity | ascii_upcase): \(.ruleId) at \(.location.file):\(.location.line):\(.location.column)\n  \(.message)\n"'
                return 1
            fi
        fi
    done
}

test_testbeds() {
    local shared_target="${CARGO_TARGET_DIR:-$(pwd)/target/testbeds}"
    for testbed in testbed/*; do
        local tb_target="$shared_target/$(basename "$testbed")"
        if [ -d "$testbed/workspace" ]; then
            for ws in "$testbed/workspace"/*; do
                if [ -f "$ws/Cargo.toml" ]; then
                    if [ "$RED_PHASE" = true ]; then
                        (cd "$ws" && CARGO_TARGET_DIR="$tb_target" cargo test --workspace --no-run && CARGO_TARGET_DIR="$tb_target" cargo clippy --workspace --all-targets -- -D warnings -A clippy::todo) || return 1
                    else
                        (cd "$ws" && CARGO_TARGET_DIR="$tb_target" cargo test --workspace && CARGO_TARGET_DIR="$tb_target" cargo clippy --workspace --all-targets -- -D warnings -D clippy::todo) || return 1
                    fi
                fi
            done
        fi
    done
}

regenerate_testbeds() {
    for testbed in testbed/*; do
        if [ -d "$testbed" ] && [ -f "$testbed/model.esdm.yaml" ]; then
            out_dir="$testbed/workspace"
            gen_features="tui"
            if [ -f "$testbed/model.infra.esdm.yaml" ]; then
                gen_features="full"
            fi
            cargo run -p chromatofoor-gen --features "$gen_features" -- \
                --input "$testbed/model.esdm.yaml" \
                --out-dir "$out_dir" \
                --chromatofoor-path ../../../../chromatofoor || return 1
        fi
    done
}

PRE_REGEN_DIFF=$(git diff HEAD -- testbed/ | sha256sum)
PRE_REGEN_STATUS=$(git status --porcelain testbed/ | sha256sum)

check_git_diffs() {
    local post_diff post_status
    post_diff=$(git diff HEAD -- testbed/ | sha256sum)
    post_status=$(git status --porcelain testbed/ | sha256sum)
    if [ "$PRE_REGEN_DIFF" != "$post_diff" ] || [ "$PRE_REGEN_STATUS" != "$post_status" ]; then
        echo "ERROR: Generated code caused git diffs! The generator is non-deterministic or generated files were out of date."
        return 1
    fi
}

run_step "Building chromatofoor-gen" build_generator
run_step "Linting ESDM models" lint_models
if [ "$RED_PHASE" = true ]; then
    run_step "Compiling & linting Red-phase testbeds" test_testbeds
else
    run_step "Testing & linting Green-phase testbeds" test_testbeds
fi
run_step "Regenerating testbeds" regenerate_testbeds
run_step "Checking for Git diffs (Non-Determinism)" check_git_diffs

echo ""
echo "=================================================="
echo "    All testbeds verified successfully! ✅"
echo "=================================================="
