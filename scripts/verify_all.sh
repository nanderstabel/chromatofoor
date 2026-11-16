#!/bin/bash
set -e

echo "=================================================="
echo "    Verify All Components"
echo "=================================================="
echo ""

run_step() {
    local step_name="$1"
    shift

    # If stdout is not a tty (e.g. redirected or running in CI), use simple fallback
    if [ ! -t 1 ]; then
        echo "=== $step_name ==="
        "$@"
        return 0
    fi

    echo -ne "  ⏳ $step_name... \n"
    local tmp_log
    tmp_log=$(mktemp)

    # Run command in background
    "$@" > "$tmp_log" 2>&1 &
    local pid=$!

    local lines_to_show=5
    local printed_lines=0

    # Get terminal width for safe truncation to avoid word wrapping
    local cols
    cols=$(tput cols 2>/dev/null || echo 80)
    local max_len=$((cols - 12))
    if [ "$max_len" -lt 20 ]; then
        max_len=20
    fi

    while kill -0 "$pid" 2>/dev/null; do
        # Clear previous lines
        if [ "$printed_lines" -gt 0 ]; then
            for ((i=0; i<printed_lines; i++)); do
                echo -ne "\033[A\033[K"
            done
        fi

        # Read last 5 lines
        local tail_output
        tail_output=$(tail -n "$lines_to_show" "$tmp_log" 2>/dev/null || true)

        printed_lines=0
        if [ -n "$tail_output" ]; then
            local lines=()
            IFS=$'\n' read -rd '' -a lines <<< "$tail_output" || true
            for line in "${lines[@]}"; do
                # Truncate line if it exceeds terminal width to prevent wrapping
                if [ "${#line}" -gt "$max_len" ]; then
                    line="${line:0:max_len}..."
                fi
                echo -e "\033[90m      | $line\033[0m"
                printed_lines=$((printed_lines + 1))
            done
        fi

        sleep 0.1
    done

    # Wait for process to end and get exit status
    local exit_code=0
    wait "$pid" || exit_code=$?

    # Clear tail lines
    if [ "$printed_lines" -gt 0 ]; then
        for ((i=0; i<printed_lines; i++)); do
            echo -ne "\033[A\033[K"
        done
    fi

    # Move cursor up to overwrite the initial prompt
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

run_step "Running workspace tests with all features" cargo test --all --all-features
run_step "Running clippy with all features (warnings as errors)" cargo clippy --all --all-features -- -D warnings
run_step "Running testbeds verification" ./scripts/verify_testbeds.sh

echo ""
echo "=================================================="
echo "    All checks passed successfully! ✅"
echo "=================================================="
