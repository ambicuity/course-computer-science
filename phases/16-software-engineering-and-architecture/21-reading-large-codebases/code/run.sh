#!/usr/bin/env bash
# Reading Large Codebases — Navigation Scripts
# Run any function: ./run.sh <function_name>
# Run all demos: ./run.sh all
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

TARGET_DIR="${1:-.}"

if [ ! -d "$TARGET_DIR" ]; then
    echo -e "${RED}Error: '$TARGET_DIR' is not a directory${RESET}"
    exit 1
fi

print_header() {
    echo ""
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo -e "${BOLD}${CYAN}  $1${RESET}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
}

print_section() {
    echo -e "${BOLD}${GREEN}▸ $1${RESET}"
}

print_info() {
    echo -e "  ${CYAN}ℹ${RESET} $1"
}

print_warn() {
    echo -e "  ${YELLOW}⚠${RESET} $1"
}

detect_languages() {
    local dir="$1"
    local langs=""
    [ -f "$dir/go.mod" ] && langs="${langs}Go "
    [ -f "$dir/Cargo.toml" ] && langs="${langs}Rust "
    [ -f "$dir/package.json" ] && langs="${langs}JavaScript/TypeScript "
    [ -f "$dir/requirements.txt" ] || [ -f "$dir/pyproject.toml" ] || [ -f "$dir/setup.py" ] && langs="${langs}Python "
    [ -f "$dir/pom.xml" ] || [ -f "$dir/build.gradle" ] && langs="${langs}Java "
    [ -f "$dir/Makefile" ] || [ -f "$dir/makefile" ] && langs="${langs}C/C++(Make) "
    [ -f "$dir/CMakeLists.txt" ] && langs="${langs}C/C++(CMake) "
    echo "$langs"
}

find_entry_points() {
    print_header "FINDING ENTRY POINTS"
    local dir="$1"

    local langs
    langs=$(detect_languages "$dir")

    if [ -z "$langs" ]; then
        print_warn "No recognized project files found. Cannot determine language."
        return
    fi

    print_info "Detected languages: $langs"
    echo ""

    echo -e "${BOLD}Main entry point candidates:${RESET}"

    if echo "$langs" | grep -q "Go"; then
        print_section "Go"
        local go_mains
        go_mains=$(find "$dir" -name 'main.go' -not -path '*/vendor/*' -not -path '*/.git/*' 2>/dev/null || true)
        if [ -n "$go_mains" ]; then
            echo "$go_mains" | while read -r f; do
                print_info "$f"
                grep -n 'func main()' "$f" 2>/dev/null || true
            done
        else
            local go_func_mains
            go_func_mains=$(grep -rn 'func main()' --include='*.go' "$dir" 2>/dev/null | grep -v vendor || true)
            if [ -n "$go_func_mains" ]; then
                echo "$go_func_mains"
            else
                print_warn "No func main() found"
            fi
        fi
        echo ""
    fi

    if echo "$langs" | grep -q "Rust"; then
        print_section "Rust"
        local rust_mains
        rust_mains=$(find "$dir" -name 'main.rs' -not -path '*/target/*' 2>/dev/null || true)
        if [ -n "$rust_mains" ]; then
            echo "$rust_mains" | while read -r f; do
                print_info "$f"
            done
        else
            print_warn "No main.rs found; check Cargo.toml for [[bin]] targets"
            grep -n '\[\[bin\]\]' "$dir/Cargo.toml" 2>/dev/null || true
        fi
        echo ""
    fi

    if echo "$langs" | grep -q "JavaScript"; then
        print_section "JavaScript/TypeScript"
        for entry in index.js index.ts app.js app.ts server.js server.ts main.js main.ts; do
            local found
            found=$(find "$dir" -name "$entry" -not -path '*/node_modules/*' -not -path '*/.git/*' -not -path '*/dist/*' 2>/dev/null || true)
            if [ -n "$found" ]; then
                echo "$found" | while read -r f; do
                    print_info "$f"
                done
            fi
        done
        local node_mains
        node_mains=$(grep -rn "if __name__" --include='*.py' "$dir" 2>/dev/null || true)
        node_mains=$(grep -rn 'require.*http\|createServer' --include='*.js' "$dir" 2>/dev/null | head -5 || true)
        if [ -n "$node_mains" ]; then
            echo "$node_mains"
        fi
        echo ""
    fi

    if echo "$langs" | grep -q "Python"; then
        print_section "Python"
        local py_mains
        py_mains=$(grep -rn 'if __name__.*==.*__main__' --include='*.py' "$dir" 2>/dev/null || true)
        if [ -n "$py_mains" ]; then
            echo "$py_mains"
        else
            print_warn "No __main__ guard found"
        fi
        echo ""
    fi
}

trace_dependencies() {
    print_header "TRACING DEPENDENCIES"
    local dir="$1"

    print_section "External Dependencies"
    if [ -f "$dir/go.mod" ]; then
        print_info "Go module: $(head -1 "$dir/go.mod")"
        local dep_count
        dep_count=$(grep -c 'require' "$dir/go.mod" 2>/dev/null || echo "0")
        print_info "Dependency blocks: $dep_count"
        echo ""
        grep '^require' "$dir/go.mod" 2>/dev/null | head -10 || true
    fi

    if [ -f "$dir/Cargo.toml" ]; then
        print_info "Rust crate: $(grep '^name' "$dir/Cargo.toml" | head -1)"
        echo ""
        grep '^\[' "$dir/Cargo.toml" | head -10 || true
        print_info "Dependencies:"
        grep -A0 'dependencies' "$dir/Cargo.toml" 2>/dev/null | head -15 || true
    fi

    if [ -f "$dir/package.json" ]; then
        print_info "Node project: $(grep '"name"' "$dir/package.json" | head -1)"
        print_info "Scripts:"
        node -e "const p=require('$dir/package.json'); Object.keys(p.scripts||{}).forEach(s=>console.log('  '+s+': '+p.scripts[s]))" 2>/dev/null || true
        print_info "Dependencies:"
        node -e "const p=require('$dir/package.json'); const d=Object.keys(p.dependencies||{}); console.log('  '+d.join(', '))" 2>/dev/null || true
    fi

    if [ -f "$dir/requirements.txt" ]; then
        print_info "Python dependencies:"
        cat "$dir/requirements.txt"
    fi

    echo ""
    print_section "Internal Module Structure"
    if command -v go &>/dev/null && [ -f "$dir/go.mod" ]; then
        print_info "Go packages:"
        find "$dir" -name '*.go' -not -path '*/vendor/*' -not -path '*/.git/*' -exec dirname {} \; 2>/dev/null | sort -u | head -20 || true
    fi

    if [ -f "$dir/Cargo.toml" ]; then
        print_info "Rust modules (from src/):"
        find "$dir/src" -name '*.rs' -not -path '*/target/*' 2>/dev/null | sort | head -20 || true
        local workspace_members
        workspace_members=$(grep 'members' "$dir/Cargo.toml" 2>/dev/null || true)
        if [ -n "$workspace_members" ]; then
            print_info "Workspace members: $workspace_members"
        fi
    fi

    if [ -f "$dir/package.json" ]; then
        print_info "JavaScript/TypeScript source files:"
        find "$dir/src" \( -name '*.js' -o -name '*.ts' \) -not -path '*/node_modules/*' 2>/dev/null | sort | head -20 || true
    fi
}

show_module_structure() {
    print_header "MODULE STRUCTURE"
    local dir="$1"

    print_section "Top-Level Directory Layout"
    local max_depth=3
    if [ -d "$dir/.git" ] || [ -f "$dir/.gitignore" ]; then
        max_depth=3
    fi

    find "$dir" -maxdepth "$max_depth" \
        -not -path '*/node_modules/*' \
        -not -path '*/.git/*' \
        -not -path '*/vendor/*' \
        -not -path '*/target/*' \
        -not -path '*/__pycache__/*' \
        -not -path '*/dist/*' \
        -not -path '*/.next/*' \
        -not -path '*/build/*' \
    | sort | head -60 || true

    echo ""
    print_section "File Counts by Extension"
    find "$dir" -type f \
        -not -path '*/node_modules/*' \
        -not -path '*/.git/*' \
        -not -path '*/vendor/*' \
        -not -path '*/target/*' \
        -not -path '*/__pycache__/*' \
        -not -path '*/dist/*' \
    2>/dev/null | sed 's/.*\.//' | sort | uniq -c | sort -rn | head -15

    echo ""
    print_section "Largest Files (potential core files)"
    find "$dir" -type f \
        -not -path '*/node_modules/*' \
        -not -path '*/.git/*' \
        -not -path '*/vendor/*' \
        -not -path '*/target/*' \
    2>/dev/null | xargs wc -l 2>/dev/null | sort -rn | head -15

    echo ""
    print_section "Most-Changed Files (Git history)"
    if [ -d "$dir/.git" ]; then
        git -C "$dir" log --format=format: --name-only 2>/dev/null | sort | uniq -c | sort -rn | head -15 || true
    else
        print_warn "Not a git repository. Cannot show change history."
    fi
}

grep_patterns() {
    print_header "GREP FOR COMMON PATTERNS"
    local dir="$1"

    print_section "TODO / FIXME / HACK Comments"
    if command -v rg &>/dev/null; then
        rg -n 'TODO|FIXME|HACK|XXX|OPTIMIZE' "$dir" 2>/dev/null | head -20 || true
    else
        grep -rn 'TODO\|FIXME\|HACK\|XXX\|OPTIMIZE' "$dir" 2>/dev/null | head -20 || true
    fi

    echo ""
    print_section "Exported / Public Symbols"
    if command -v rg &>/dev/null; then
        rg -n '^(func|type|var|const) [A-Z]' "$dir" 2>/dev/null | head -20 || true
        rg -n '^(export |export default |export function |export class |export const )' "$dir" 2>/dev/null | head -20 || true
        rg -n '^(pub fn|pub struct|pub enum|pub trait|pub const|pub static)' "$dir" 2>/dev/null | head -20 || true
    else
        grep -rn '^\(func\|type\|var\|const\) [A-Z]' "$dir" 2>/dev/null | head -20 || true
    fi

    echo ""
    print_section "Interface Definitions"
    if command -v rg &>/dev/null; then
        rg -n 'type.*interface' "$dir" 2>/dev/null | head -20 || true
        rg -n 'protocol ' "$dir" 2>/dev/null | head -10 || true
        rg -n 'interface ' "$dir" 2>/dev/null | head -10 || true
    else
        grep -rn 'type.*interface' "$dir" 2>/dev/null | head -20 || true
    fi

    echo ""
    print_section "Error Definitions"
    if command -v rg &>/dev/null; then
        rg -n 'Error|error|Err' "$dir" --type-add 'src:*.{go,rs,py,ts,js,java}' -t src 2>/dev/null | head -20 || true
    else
        grep -rn 'Error' "$dir" --include='*.go' --include='*.rs' --include='*.py' --include='*.ts' 2>/dev/null | head -20 || true
    fi

    echo ""
    print_section "Test Files"
    find "$dir" \( -name '*_test.go' -o -name 'test_*.py' -o -name '*.test.ts' -o -name '*.spec.ts' -o -name '*Test.java' -o -name '*Tests.swift' \) \
        -not -path '*/node_modules/*' \
        -not -path '*/vendor/*' \
        -not -path '*/target/*' \
    2>/dev/null | head -20 || true

    echo ""
    print_section "Configuration Files"
    find "$dir" -maxdepth 2 \
        \( -name '*.yaml' -o -name '*.yml' -o -name '*.toml' -o -name '*.json' -o -name '*.env*' -o -name '.env*' -o -name 'Dockerfile*' -o -name 'docker-compose*' \) \
        -not -path '*/node_modules/*' \
        -not -path '*/.git/*' \
    2>/dev/null | sort || true
}

show_git_history() {
    print_header "GIT HISTORY (Recent Activity)"
    local dir="$1"

    if [ ! -d "$dir/.git" ]; then
        print_warn "Not a git repository. Cannot show history."
        print_info "Initialize with: cd $dir && git init"
        return
    fi

    print_section "Last 20 Commits"
    git -C "$dir" log --oneline -20 2>/dev/null || true

    echo ""
    print_section "Commits by Author (Top 10)"
    git -C "$dir" shortlog -sn --all 2>/dev/null | head -10 || true

    echo ""
    print_section "Most Active Files (Last 100 Commits)"
    git -C "$dir" log --format=format: --name-only -100 2>/dev/null | sort | uniq -c | sort -rn | head -15 || true

    echo ""
    print_section "Recent Branches"
    git -C "$dir" branch -a --sort=-committerdate 2>/dev/null | head -10 || true

    echo ""
    print_section "File Blame Summary (Most-touched files)"
    print_info "Run 'git blame <file>' on any file to see who wrote each line and when."
    print_info "Run 'git log --follow -- <file>' to see a file's full history including renames."
}

show_test_overview() {
    print_header "TEST OVERVIEW"
    local dir="$1"

    print_section "Test File Locations"
    local test_count
    test_count=$(find "$dir" \( -name '*_test.go' -o -name 'test_*.py' -o -name '*.test.ts' -o -name '*.spec.ts' -o -name '*Test.java' \) \
        -not -path '*/node_modules/*' \
        -not -path '*/vendor/*' \
        -not -path '*/target/*' \
    2>/dev/null | wc -l | tr -d ' ')
    print_info "Found $test_count test files"

    if [ "$test_count" -eq 0 ]; then
        print_warn "No test files found. This may indicate low test coverage."
        return
    fi

    echo ""
    find "$dir" \( -name '*_test.go' -o -name 'test_*.py' -o -name '*.test.ts' -o -name '*.spec.ts' \) \
        -not -path '*/node_modules/*' \
        -not -path '*/vendor/*' \
    2>/dev/null | sort | head -20 || true

    echo ""
    print_section "Test Function Count"
    local go_tests=0 py_tests=0 js_tests=0
    if command -v rg &>/dev/null; then
        go_tests=$(rg -c 'func Test' "$dir" --include='*_test.go' 2>/dev/null | awk -F: '{s+=$2} END{print s}' || echo 0)
        py_tests=$(rg -c 'def test_' "$dir" --include='test_*.py' --include='*_test.py' 2>/dev/null | awk -F: '{s+=$2} END{print s}' || echo 0)
        js_tests=$(rg -c '(test\(|it\(|describe\()' "$dir" --include='*.test.ts' --include='*.spec.ts' 2>/dev/null | awk -F: '{s+=$2} END{print s}' || echo 0)
    else
        go_tests=$(grep -rn 'func Test' "$dir" --include='*_test.go' 2>/dev/null | wc -l || echo 0)
        py_tests=$(grep -rn 'def test_' "$dir" --include='test_*.py' --include='*_test.py' 2>/dev/null | wc -l || echo 0)
        js_tests=$(grep -rn 'test\(' "$dir" --include='*.test.ts' --include='*.spec.ts' 2>/dev/null | wc -l || echo 0)
    fi
    print_info "Go tests: $go_tests"
    print_info "Python tests: $py_tests"
    print_info "JavaScript/TypeScript tests: $js_tests"

    echo ""
    print_section "Test Run Commands"
    if [ -f "$dir/go.mod" ]; then
        print_info "Go: go test ./..."
    fi
    if [ -f "$dir/Cargo.toml" ]; then
        print_info "Rust: cargo test"
    fi
    if [ -f "$dir/package.json" ]; then
        print_info "Node: npm test (or check package.json scripts)"
    fi
    if [ -f "$dir/pytest.ini" ] || [ -f "$dir/pyproject.toml" ] || [ -d "$dir/tests" ]; then
        print_info "Python: pytest"
    fi
}

show_build_system() {
    print_header "BUILD SYSTEM"
    local dir="$1"

    print_section "Detected Build Files"
    for f in Makefile makefile Cargo.toml package.json go.mod pyproject.toml CMakeLists.txt build.gradle pom.xml build.sbt justfile Taskfile.yml; do
        if [ -f "$dir/$f" ]; then
            print_info "Found: $f"
        fi
    done

    echo ""
    print_section "Available Build Commands"
    if [ -f "$dir/Makefile" ] || [ -f "$dir/makefile" ]; then
        print_info "Makefile targets:"
        make -C "$dir" -pn 2>/dev/null | grep '^[a-zA-Z0-9_-]*:' | sed 's/:.*//' | head -20 || true
    fi

    if [ -f "$dir/Cargo.toml" ]; then
        print_info "Rust: cargo build / cargo test / cargo run"
    fi

    if [ -f "$dir/package.json" ]; then
        print_info "Node scripts:"
        node -e "const p=require('$dir/package.json'); Object.keys(p.scripts||{}).forEach(s=>console.log('  npm run '+s))" 2>/dev/null || true
    fi

    if [ -f "$dir/go.mod" ]; then
        print_info "Go: go build ./... / go test ./... / go run ./cmd/..."
    fi

    if [ -f "$dir/pyproject.toml" ]; then
        print_info "Python: check pyproject.toml for build system and scripts"
    fi
}

show_summary() {
    print_header "CODEBASE SUMMARY"
    local dir="$1"

    local langs
    langs=$(detect_languages "$dir")
    print_info "Languages: ${langs:-unknown}"

    local total_files
    total_files=$(find "$dir" -type f \
        -not -path '*/node_modules/*' \
        -not -path '*/.git/*' \
        -not -path '*/vendor/*' \
        -not -path '*/target/*' \
        -not -path '*/__pycache__/*' \
    2>/dev/null | wc -l | tr -d ' ')
    print_info "Total files: $total_files"

    local total_lines
    total_lines=$(find "$dir" -type f \
        -not -path '*/node_modules/*' \
        -not -path '*/.git/*' \
        -not -path '*/vendor/*' \
        -not -path '*/target/*' \
        -not -path '*/__pycache__/*' \
    2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}')
    print_info "Total lines: ${total_lines:-unknown}"

    if [ -d "$dir/.git" ]; then
        local commit_count
        commit_count=$(git -C "$dir" rev-list --count HEAD 2>/dev/null || echo "unknown")
        local contributors
        contributors=$(git -C "$dir" shortlog -sn --all 2>/dev/null | wc -l | tr -d ' ')
        print_info "Git commits: $commit_count"
        print_info "Contributors: $contributors"
    fi

    local test_count
    test_count=$(find "$dir" \( -name '*_test.go' -o -name 'test_*.py' -o -name '*.test.ts' -o -name '*.spec.ts' -o -name '*Test.java' \) \
        -not -path '*/node_modules/*' \
        -not -path '*/vendor/*' \
        -not -path '*/target/*' \
    2>/dev/null | wc -l | tr -d ' ')
    print_info "Test files: $test_count"

    echo ""
    print_section "Recommended Reading Strategy"
    if [ "$total_lines" -lt 1000 ] 2>/dev/null; then
        print_info "Small codebase (< 1K lines): Read it all. Start from entry point."
    elif [ "$total_lines" -lt 50000 ] 2>/dev/null; then
        print_info "Medium codebase (1K-50K lines): README → build files → entry point → one module deep dive."
    elif [ "$total_lines" -lt 500000 ] 2>/dev/null; then
        print_info "Large codebase (50K-500K lines): README → architecture docs → one subsystem deep dive."
    else
        print_info "Very large codebase (500K+ lines): Architecture docs → one subsystem as its own codebase."
    fi
}

case "${2:-all}" in
    entry-points|entry)
        find_entry_points "$TARGET_DIR"
        ;;
    dependencies|deps)
        trace_dependencies "$TARGET_DIR"
        ;;
    modules|structure)
        show_module_structure "$TARGET_DIR"
        ;;
    grep|patterns)
        grep_patterns "$TARGET_DIR"
        ;;
    git|history)
        show_git_history "$TARGET_DIR"
        ;;
    tests)
        show_test_overview "$TARGET_DIR"
        ;;
    build)
        show_build_system "$TARGET_DIR"
        ;;
    summary)
        show_summary "$TARGET_DIR"
        ;;
    all)
        show_summary "$TARGET_DIR"
        find_entry_points "$TARGET_DIR"
        trace_dependencies "$TARGET_DIR"
        show_module_structure "$TARGET_DIR"
        show_test_overview "$TARGET_DIR"
        grep_patterns "$TARGET_DIR"
        show_git_history "$TARGET_DIR"
        show_build_system "$TARGET_DIR"
        ;;
    *)
        echo "Usage: $0 <directory> <command>"
        echo ""
        echo "Commands:"
        echo "  entry-points    Find main() and entry point candidates"
        echo "  dependencies    Trace external and internal dependencies"
        echo "  modules         Show module structure and largest files"
        echo "  grep            Search for common patterns (TODO, exports, etc.)"
        echo "  git             Show git history and activity"
        echo "  tests           Show test file locations and counts"
        echo "  build           Show build system and available commands"
        echo "  summary         Show codebase size and recommended reading strategy"
        echo "  all             Run all commands (default)"
        exit 1
        ;;
esac