#!/usr/bin/env bash
set -euo pipefail

###############################################################################
# Lesson 04: Flamegraphs, Hotspots, and Reading Stacks
#
# This script demonstrates the complete flamegraph generation pipeline:
#   perf record → perf script → stackcollapse → flamegraph.pl
#
# Plus a self-contained SVG generator so students can produce flamegraphs
# without installing Brendan Gregg's toolchain.
#
# Sections:
#   1. CPU flamegraph generation (full pipeline)
#   2. Off-CPU flamegraph generation
#   3. Differential flamegraph (before/after comparison)
#   4. Self-contained SVG flamegraph generator (no external tools)
#   5. Stackcollapse parser (converts perf script output to folded format)
#
# Usage:
#   ./run.sh [section]
#   ./run.sh          # run all demos with synthetic data
#   ./run.sh cpu      # CPU flamegraph demo
#   ./run.sh offcpu   # off-CPU flamegraph demo
#   ./run.sh diff     # differential flamegraph demo
#   ./run.sh self     # self-contained SVG generator demo
#   ./run.sh collapse # stackcollapse parser demo
###############################################################################

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/../outputs"
mkdir -p "${OUTPUT_DIR}"

###############################################################################
# Utility: create synthetic folded-stack data for demos
###############################################################################
generate_synthetic_cpu_stacks() {
    cat <<'STACKS'
main;parse;tokenize;scan_char 300
main;parse;tokenize;scan_string 180
main;parse;validate;check_type 120
main;parse;validate;check_range 80
main;compress;lz4;hash_match 250
main;compress;lz4;literal_copy 150
main;compress;zstd;find_match 60
main;compress;zstd;encode_sequence 40
main;io;read_block;kernel_read 120
main;io;write_block;kernel_write 90
main;alloc;malloc_large 60
main;alloc;malloc_small 40
main;gc;mark_sweep;trace_refs 50
main;gc;mark_sweep;compact 30
STACKS
}

generate_synthetic_offcpu_stacks() {
    cat <<'STACKS'
main;io;read_block;futex_wait 450
main;io;read_block;epoll_wait 200
main;io;write_block;futex_wait 150
main;compress;lz4;mutex_lock 80
main;parse;tokenize;cond_wait 60
main;alloc;malloc_large;mmap_brk 40
main;gc;mark_sweep;stop_the_world 20
STACKS
}

generate_synthetic_after_stacks() {
    cat <<'STACKS'
main;parse;tokenize;scan_char 180
main;parse;tokenize;scan_string 100
main;parse;validate;check_type 120
main;parse;validate;check_range 80
main;compress;lz4;hash_match 120
main;compress;lz4;literal_copy 80
main;compress;zstd;find_match 60
main;compress;zstd;encode_sequence 40
main;io;read_block;kernel_read 130
main;io;write_block;kernel_write 95
main;alloc;malloc_large 60
main;alloc;malloc_small 40
main;gc;mark_sweep;trace_refs 50
main;gc;mark_sweep;compact 30
STACKS
}

###############################################################################
# Section 1: CPU Flamegraph Generation (Full Pipeline)
#
# The standard pipeline:
#   1. perf record   — sample stack traces from the kernel
#   2. perf script   — dump recorded samples as text
#   3. stackcollapse — fold multi-line stacks into semicolon format
#   4. flamegraph.pl — generate SVG from folded stacks
#
# In production you would run:
#   perf record -F 99 -g -- ./your_program
#
# For this demo we use synthetic folded-stack data so it works anywhere.
###############################################################################
demo_cpu_flamegraph() {
    echo "=== Section 1: CPU Flamegraph Generation ==="
    echo ""
    echo "In production, the full pipeline is:"
    echo "  \$ perf record -F 99 -g -- ./your_program"
    echo "  \$ perf script | stackcollapse-perf.pl | flamegraph.pl > cpu.svg"
    echo ""
    echo "Step 1: perf record -F 99 -g -- ./your_program"
    echo "  -F 99: sample at 99 Hz (avoids lock-step with timer interrupts)"
    echo "  -g: record call graphs (stack traces)"
    echo "  --: separator before the command to profile"
    echo ""
    echo "Step 2: perf script"
    echo "  Converts binary perf.data to text format:"
    echo ""
    echo "    nginx 12345 [000] 1234.567: 100000 cpu-clock:"
    echo "        7f8a9b123456 ngx_http_process_request"
    echo "        7f8a9b234567 ngx_http_core_run_phases"
    echo "        7f8a9b345678 ngx_http_core_content_phase"
    echo ""
    echo "Step 3: stackcollapse-perf.pl"
    echo "  Converts multi-line stacks to folded format:"
    echo ""
    echo "    main;ngx_http_process_request;ngx_http_core_content_phase 42"
    echo ""
    echo "Step 4: flamegraph.pl"
    echo "  Generates an interactive SVG flamegraph."
    echo ""
    echo "--- For this demo, generating CPU flamegraph from synthetic data ---"

    local folded_file="${OUTPUT_DIR}/cpu_folded.stacks"
    local svg_file="${OUTPUT_DIR}/cpu_flamegraph.svg"

    generate_synthetic_cpu_stacks > "${folded_file}"
    echo "Wrote folded stacks to ${folded_file}"

    generate_svg_flamegraph "${folded_file}" "${svg_file}" "CPU Flamegraph (Synthetic)"
    echo "Generated CPU flamegraph: ${svg_file}"
    echo ""
}

###############################################################################
# Section 2: Off-CPU Flamegraph Generation
#
# Off-CPU flamegraphs show where threads are BLOCKED, not where they are running.
# Use these when CPU usage is low but latency is high.
#
# In production:
#   perf record -e sched:sched_stat_sleep -g -- ./your_program
#   # Or with bcc:
#   offcputime -p <pid> > offcpu.stacks
#
# Then stackcollapse and flamegraph.pl as usual.
###############################################################################
demo_offcpu_flamegraph() {
    echo "=== Section 2: Off-CPU Flamegraph Generation ==="
    echo ""
    echo "Off-CPU flamegraphs show where threads are WAITING, not running."
    echo "Use these when CPU is low but latency is high."
    echo ""
    echo "Production commands:"
    echo "  # Method 1: perf with sched events"
    echo "  \$ perf record -e sched:sched_stat_sleep -g -- ./your_program"
    echo "  \$ perf script | stackcollapse-perf.pl | flamegraph.pl --color=io > offcpu.svg"
    echo ""
    echo "  # Method 2: bcc offcputime (better resolution)"
    echo "  \$ offcputime -p <pid> > offcpu.stacks"
    echo "  \$ stackcollapse-bpftrace.pl offcpu.stacks | flamegraph.pl --color=io > offcpu.svg"
    echo ""
    echo "--- For this demo, generating off-CPU flamegraph from synthetic data ---"

    local folded_file="${OUTPUT_DIR}/offcpu_folded.stacks"
    local svg_file="${OUTPUT_DIR}/offcpu_flamegraph.svg"

    generate_synthetic_offcpu_stacks > "${folded_file}"
    echo "Wrote folded stacks to ${folded_file}"

    generate_svg_flamegraph "${folded_file}" "${svg_file}" "Off-CPU Flamegraph (Synthetic)" "blue"
    echo "Generated off-CPU flamegraph: ${svg_file}"
    echo ""
}

###############################################################################
# Section 3: Differential Flamegraph (Before/After Comparison)
#
# Differential flamegraphs compare two flamegraphs and color bars by whether
# each function got more (red) or fewer (blue) samples.
#
# Production pipeline:
#   1. Generate "before" folded stacks
#   2. Optimize
#   3. Generate "after" folded stacks
#   4. difffolded.pl before.stacks after.stacks | flamegraph.pl --negate
#
# The --negate flag colors increases as red (bad) and decreases as blue (good).
###############################################################################
demo_differential_flamegraph() {
    echo "=== Section 3: Differential Flamegraph ==="
    echo ""
    echo "Differential flamegraphs compare before/after optimization."
    echo "Red = regression (more samples), Blue = improvement (fewer samples)."
    echo ""
    echo "Production pipeline:"
    echo "  # Before optimization"
    echo "  \$ perf record -F 99 -g -- ./your_program"
    echo "  \$ perf script | stackcollapse-perf.pl > before.stacks"
    echo ""
    echo "  # ... apply optimization ..."
    echo ""
    echo "  # After optimization"
    echo "  \$ perf record -F 99 -g -- ./your_program"
    echo "  \$ perf script | stackcollapse-perf.pl > after.stacks"
    echo ""
    echo "  # Generate differential flamegraph"
    echo "  \$ difffolded.pl before.stacks after.stacks | flamegraph.pl --negate > diff.svg"
    echo ""
    echo "--- For this demo, computing differential from synthetic data ---"

    local before_file="${OUTPUT_DIR}/before_folded.stacks"
    local after_file="${OUTPUT_DIR}/after_folded.stacks"
    local diff_folded="${OUTPUT_DIR}/diff_folded.stacks"
    local svg_file="${OUTPUT_DIR}/diff_flamegraph.svg"

    generate_synthetic_cpu_stacks > "${before_file}"
    generate_synthetic_after_stacks > "${after_file}"

    compute_differential "${before_file}" "${after_file}" > "${diff_folded}"

    local before_total after_total
    before_total=$(awk '{s+=$2} END{print s}' "${before_file}")
    after_total=$(awk '{s+=$2} END{print s}' "${after_file}")
    echo "Before (total samples): ${before_total}"
    echo "After  (total samples): ${after_total}"
    echo "Wrote differential folded stacks to ${diff_folded}"

    generate_svg_flamegraph "${before_file}" "${svg_file}" "Differential Flamegraph (Synthetic)" "diff" "${after_file}"
    echo "Generated differential flamegraph: ${svg_file}"
    echo ""
}

# Compute differential between two folded-stack files.
# Output format: stack_label delta_count
# Positive delta = regression, negative = improvement.
compute_differential() {
    local before="$1"
    local after="$2"

    awk '
    BEGIN { }
    NR==FNR { before[$1] = $2; next }
    { after[$1] = $2 }
    END {
        for (k in before) {
            a = (k in after) ? after[k] : 0
            delta = a - before[k]
            if (delta != 0) print k, delta
        }
        for (k in after) {
            if (!(k in before)) print k, after[k]
        }
    }
    ' "${before}" "${after}"
}

###############################################################################
# Section 4: Self-Contained SVG Flamegraph Generator
#
# Generates a flamegraph SVG from folded-stack input WITHOUT requiring
# Brendan Gregg's FlameGraph.pl or any external tools.
#
# Input format (folded stacks):
#   main;func_a;func_b 150
#   main;func_a;func_c 80
#
# Output: SVG file with interactive bars (hover for details)
#
# This is the core algorithm:
#   1. Parse folded stacks into a tree of (name, count) nodes
#   2. Walk the tree; for each node, compute x position from cumulative width
#   3. Render each node as a colored rectangle with text label
#   4. Add JavaScript for hover/click interactivity
#
# The awk layout engine is written to a temp file to avoid quoting issues
# with large inline scripts inside bash.
###############################################################################
generate_svg_flamegraph() {
    local input_file="$1"
    local output_file="$2"
    local title="${3:-Flamegraph}"
    local color_scheme="${4:-warm}"
    local diff_file="${5:-}"

    local tmpdir
    tmpdir=$(mktemp -d)
    # Cleanup on exit (only remove our tmpdir, not any existing one)
    _cleanup() {
        rm -rf "${tmpdir}" 2>/dev/null || true
    }

    local awk_script="${tmpdir}/layout.awk"
    cat > "${awk_script}" << 'AWKEOF'
BEGIN {
    total_count = 0
    FS = " "
}
{
    stack = ""
    count = 0
    n = split($0, parts, " ")
    if (n >= 2) {
        for (i = 1; i < n; i++) {
            if (i > 1) stack = stack " "
            stack = stack parts[i]
        }
        count = parts[n] + 0
    }
    nframes = split(stack, frames, ";")
    if (count == 0) next
    total_count += count
    for (d = 1; d <= nframes; d++) {
        path = frames[1]
        for (i = 2; i <= d; i++) {
            path = path ";" frames[i]
        }
        if (!(path in node_count)) {
            node_count[path] = 0
        }
        node_count[path] += count
    }
}
END {
    n = 0
    for (path in node_count) {
        n++
        paths[n] = path
        counts[n] = node_count[path]
    }
    for (i = 1; i <= n; i++) {
        depth = 1
        p = paths[i]
        l = length(p)
        for (c = 1; c <= l; c++) {
            if (substr(p, c, 1) == ";") depth++
        }
        depths[i] = depth
    }
    for (i = 1; i <= n; i++) {
        for (j = i + 1; j <= n; j++) {
            swap = 0
            if (depths[i] > depths[j]) swap = 1
            else if (depths[i] == depths[j] && paths[i] > paths[j]) swap = 1
            if (swap) {
                tmp = paths[i]; paths[i] = paths[j]; paths[j] = tmp
                tmp = counts[i]; counts[i] = counts[j]; counts[j] = tmp
                tmp = depths[i]; depths[i] = depths[j]; depths[j] = tmp
            }
        }
    }
    for (i = 1; i <= n; i++) {
        p = paths[i]
        last_semi = 0
        for (c = length(p); c >= 1; c--) {
            if (substr(p, c, 1) == ";") {
                last_semi = c
                break
            }
        }
        if (last_semi > 0) {
            parent = substr(p, 1, last_semi - 1)
        } else {
            parent = ""
        }
        parents[i] = parent
    }
    for (i = 1; i <= n; i++) {
        w = (counts[i] / total_count) * 1000
        widths[i] = w
    }
    for (i = 1; i <= n; i++) {
        x_pos[i] = 0
    }
    cumx = 0
    for (i = 1; i <= n; i++) {
        if (depths[i] == 1) {
            x_pos[i] = cumx
            cumx += widths[i]
        }
    }
    for (d = 2; d <= 100; d++) {
        cumx_for_parent[""] = 0
        for (i = 1; i <= n; i++) {
            if (depths[i] == d) {
                p = parents[i]
                if (!(p in cumx_for_parent)) {
                    for (j = 1; j <= n; j++) {
                        if (paths[j] == p) {
                            cumx_for_parent[p] = x_pos[j]
                            break
                        }
                    }
                }
                x_pos[i] = cumx_for_parent[p]
                cumx_for_parent[p] += widths[i]
            }
        }
    }
    max_depth = 0
    for (i = 1; i <= n; i++) {
        if (depths[i] > max_depth) max_depth = depths[i]
    }
    print "METADATA " total_count " " max_depth
    for (i = 1; i <= n; i++) {
        p = paths[i]
        name = p
        for (c = length(p); c >= 1; c--) {
            if (substr(p, c, 1) == ";") {
                name = substr(p, c + 1)
                break
            }
        }
        printf "NODE %d %.2f %.2f %s %d %s\n", depths[i], x_pos[i], widths[i], name, counts[i], paths[i]
    }
}
AWKEOF

    local layout_file="${tmpdir}/layout.txt"
    awk -f "${awk_script}" "${input_file}" > "${layout_file}"

    local total_count max_depth img_width img_height bar_height
    read -r _ total_count max_depth < <(head -1 "${layout_file}")

    img_width=1200
    bar_height=18
    img_height=$(( (max_depth + 3) * bar_height + 40 ))

    {
        cat <<SVGHEAD
<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${img_width}" height="${img_height}" viewBox="0 0 1000 $(( max_depth * bar_height + 60 ))">
<style>
  rect { stroke: #999; stroke-width: 0.5; cursor: pointer; }
  rect:hover { stroke: #000; stroke-width: 2; }
  text { font-family: monospace; font-size: 11px; fill: #333; pointer-events: none; }
  .title { font-size: 14px; font-weight: bold; fill: #000; }
</style>
<script type="text/javascript"><![CDATA[
function details(evt) {
  var r = evt.target;
  var info = document.getElementById("info");
  info.textContent = r.getAttribute("name") + " (" + r.getAttribute("samples") + " samples, " + r.getAttribute("pct") + "%)";
}
]]></script>
<rect x="0" y="0" width="1000" height="${img_height}" fill="#f8f8f8" rx="4"/>
<text x="500" y="16" text-anchor="middle" class="title">${title}</text>
<text id="info" x="500" y="$(( max_depth * bar_height + 50 ))" text-anchor="middle" style="font-size:12px;fill:#666;">Hover over a bar for details</text>
SVGHEAD

        local line depth x_pos_val width name samples path pct
        tail -n +2 "${layout_file}" | while IFS=' ' read -r type depth x_pos_val width name samples path; do
            [ "$type" = "NODE" ] || continue
            pct=$(echo "scale=1; ${samples} * 100 / ${total_count}" | bc 2>/dev/null || echo "0")

            local fill="#e87d2f"
            if [ "$color_scheme" = "blue" ]; then
                local hash=0
                hash=$(echo "$name" | cksum | cut -d' ' -f1)
                local blue_val=$(( 100 + (hash % 100) ))
                fill="#4466${blue_val}"
            elif [ "$color_scheme" = "diff" ]; then
                fill="#cccccc"
            else
                local hash=0
                hash=$(echo "$name" | cksum | cut -d' ' -f1)
                local red=$(( 200 + (hash % 55) ))
                local green=$(( 100 + (hash % 80) ))
                local blue=$(( 20 + (hash % 40) ))
                fill=$(printf "#%02x%02x%02x" "$red" "$green" "$blue")
            fi

            local y=$(( (depth - 1) * bar_height + 25 ))

            local text_len=${#name}
            local chars_fit
            chars_fit=$(echo "scale=0; ${width} / 12" | bc 2>/dev/null || echo "0")
            chars_fit=${chars_fit%.*}
            local display_name="${name}"
            if [ "${chars_fit}" -gt 0 ] && [ "${text_len}" -gt "${chars_fit}" ]; then
                display_name="${name:0:${chars_fit}}.."
            fi

            local x_int float_x_pos_val
            float_x_pos_val=$(echo "${x_pos_val}" | bc 2>/dev/null || echo "${x_pos_val}")
            x_int=$(printf "%.0f" "${x_pos_val}" 2>/dev/null || echo "${x_pos_val%.*}")

            echo "  <rect x=\"${x_pos_val}\" y=\"${y}\" width=\"${width}\" height=\"${bar_height}\" fill=\"${fill}\" name=\"${path}\" samples=\"${samples}\" pct=\"${pct}\" onmouseover=\"details(evt)\"/>"
            if [ "$(echo "${width} > 30" | bc 2>/dev/null || echo 0)" -eq 1 ]; then
                local text_x=$(( x_int + 2 ))
                local text_y=$(( y + 13 ))
                echo "  <text x=\"${text_x}\" y=\"${text_y}\">${display_name}</text>"
            fi
        done

        echo "</svg>"
    } > "${output_file}"

    echo "SVG flamegraph written to ${output_file}"
    _cleanup
}

###############################################################################
# Section 5: Stackcollapse Parser
#
# Converts perf script output into the folded-stack format used by
# flamegraph generators.
#
# Input (perf script format):
#   nginx 12345 [000] 1234.567: 100000 cpu-clock:
#       7f8a9b123456 ngx_http_process_request [nginx]
#       7f8a9b234567 ngx_http_core_run_phases [nginx]
#       7f8a9b345678 ngx_http_core_content_phase [nginx]
#
# Output (folded format):
#   main;ngx_http_process_request;ngx_http_core_content_phase 1
#
# This is a simplified version of stackcollapse-perf.pl.
###############################################################################
demo_stackcollapse() {
    echo "=== Section 5: Stackcollapse Parser ==="
    echo ""
    echo "The stackcollapse step converts perf script output into folded-stack format."
    echo ""
    echo "Input (perf script multi-line format):"
    echo "  nginx 12345 [000] 1234.567: 100000 cpu-clock:"
    echo "      7f8a9b123456 ngx_http_process_request [nginx]"
    echo "      7f8a9b234567 ngx_http_core_run_phases [nginx]"
    echo ""
    echo "Output (folded format):"
    echo "  main;ngx_http_process_request;ngx_http_core_run_phases 1"
    echo ""
    echo "--- Parsing synthetic perf script data ---"

    local perf_file="${OUTPUT_DIR}/sample_perf_script.txt"
    local folded_file="${OUTPUT_DIR}/sample_folded.stacks"

    cat > "${perf_file}" <<'PERF'
process1 1234 [000] 100.001: 100000 cpu-clock:
	7f0001 main [process1]
	7f0002 parse [process1]
	7f0003 tokenize [process1]

process1 1234 [000] 100.002: 100000 cpu-clock:
	7f0001 main [process1]
	7f0002 parse [process1]
	7f0003 tokenize [process1]

process1 1234 [000] 100.003: 100000 cpu-clock:
	7f0001 main [process1]
	7f0002 parse [process1]
	7f0004 validate [process1]

process1 1234 [000] 100.004: 100000 cpu-clock:
	7f0001 main [process1]
	7f0002 compress [process1]
	7f0005 lz4 [process1]

process1 1234 [000] 100.005: 100000 cpu-clock:
	7f0001 main [process1]
	7f0002 compress [process1]
	7f0005 lz4 [process1]
PERF

    stackcollapse_perf "${perf_file}" > "${folded_file}"

    echo "Folded stacks:"
    cat "${folded_file}"
    echo ""
    echo "Stackcollapse complete. Output at ${folded_file}"
    echo ""
}

# Convert perf script output to folded stacks
stackcollapse_perf() {
    local input_file="$1"
    local tmpdir
    tmpdir=$(mktemp -d)
    local awk_script="${tmpdir}/stackcollapse.awk"

    cat > "${awk_script}" << 'AWKEOF'
/^[a-zA-Z]/ {
    if (stack != "") {
        counts[stack]++
    }
    stack = ""
    next
}
/^\t/ {
    line = $0
    gsub(/^[ \t]+/, "", line)
    gsub(/^[0-9a-f]+ /, "", line)
    gsub(/ \[.*\]$/, "", line)
    gsub(/[ \t]+$/, "", line)
    if (line != "") {
        if (stack == "") {
            stack = line
        } else {
            stack = stack ";" line
        }
    }
    next
}
END {
    if (stack != "") {
        counts[stack]++
    }
    n = 0
    for (k in counts) {
        n++
        keys[n] = k
    }
    for (i = 1; i <= n; i++) {
        for (j = i + 1; j <= n; j++) {
            if (keys[i] > keys[j]) {
                tmp = keys[i]
                keys[i] = keys[j]
                keys[j] = tmp
            }
        }
    }
    for (i = 1; i <= n; i++) {
        print keys[i] " " counts[keys[i]]
    }
}
AWKEOF

    awk -f "${awk_script}" "${input_file}"
    rm -rf "${tmpdir}"
}

###############################################################################
# Main Entry Point
###############################################################################
main() {
    local section="${1:-all}"

    echo "Lesson 04: Flamegraphs, Hotspots, and Reading Stacks"
    echo "====================================================="
    echo ""

    case "${section}" in
        cpu)
            demo_cpu_flamegraph
            ;;
        offcpu)
            demo_offcpu_flamegraph
            ;;
        diff)
            demo_differential_flamegraph
            ;;
        self)
            echo "Generating self-contained flamegraph from synthetic data..."
            local folded_file="${OUTPUT_DIR}/cpu_folded.stacks"
            generate_synthetic_cpu_stacks > "${folded_file}"
            generate_svg_flamegraph "${folded_file}" "${OUTPUT_DIR}/cpu_flamegraph.svg" "Self-Contained CPU Flamegraph"
            echo "Done. See ${OUTPUT_DIR}/cpu_flamegraph.svg"
            ;;
        collapse)
            demo_stackcollapse
            ;;
        all)
            demo_cpu_flamegraph
            demo_offcpu_flamegraph
            demo_differential_flamegraph
            demo_stackcollapse
            echo "All demos complete! Generated files in ${OUTPUT_DIR}/"
            echo ""
            echo "Files created:"
            ls -la "${OUTPUT_DIR}/"
            ;;
        *)
            echo "Usage: $0 [cpu|offcpu|diff|self|collapse|all]"
            echo ""
            echo "Sections:"
            echo "  cpu      - CPU flamegraph generation pipeline"
            echo "  offcpu   - Off-CPU flamegraph generation"
            echo "  diff     - Differential flamegraph (before/after)"
            echo "  self     - Self-contained SVG generator only"
            echo "  collapse - Stackcollapse parser demo"
            echo "  all      - Run all demos (default)"
            exit 1
            ;;
    esac
}

main "$@"