#!/usr/bin/env bash
set -euo pipefail

PIPELINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RESULTS_DIR="${PIPELINE_ROOT}/outputs"
PIPELINE_LOG="${RESULTS_DIR}/pipeline.log"
EXIT_CODE=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

STAGE_LINT="lint"
STAGE_TEST="test"
STAGE_BUILD="build"
STAGE_DEPLOY="deploy"
STAGE_VALIDATE="validate"

TOTAL_STAGES=5
PASSED_STAGES=0
FAILED_STAGES=0
SKIPPED_STAGES=0

log() {
    local stage="$1"
    local message="$2"
    echo "[$(date '+%H:%M:%S')] [${stage}] ${message}" | tee -a "${PIPELINE_LOG}"
}

log_info() {
    local message="$1"
    echo -e "${CYAN}[INFO]${NC} ${message}"
    echo "[$(date '+%H:%M:%S')] [INFO] ${message}" >> "${PIPELINE_LOG}"
}

log_success() {
    local message="$1"
    echo -e "${GREEN}[PASS]${NC} ${message}"
    echo "[$(date '+%H:%M:%S')] [PASS] ${message}" >> "${PIPELINE_LOG}"
}

log_error() {
    local message="$1"
    echo -e "${RED}[FAIL]${NC} ${message}" >&2
    echo "[$(date '+%H:%M:%S')] [FAIL] ${message}" >> "${PIPELINE_LOG}"
}

log_warn() {
    local message="$1"
    echo -e "${YELLOW}[WARN]${NC} ${message}"
    echo "[$(date '+%H:%M:%S')] [WARN] ${message}" >> "${PIPELINE_LOG}"
}

run_stage() {
    local stage_name="$1"
    shift
    local start_time
    start_time=$(date +%s)

    log "${stage_name}" "Starting stage: ${stage_name}"
    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  STAGE: ${stage_name}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    if "$@"; then
        local end_time
        end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "${stage_name} completed in ${duration}s"
        PASSED_STAGES=$((PASSED_STAGES + 1))
        return 0
    else
        local exit_code=$?
        local end_time
        end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_error "${stage_name} failed after ${duration}s (exit code: ${exit_code})"
        FAILED_STAGES=$((FAILED_STAGES + 1))
        EXIT_CODE=1
        return ${exit_code}
    fi
}

stage_lint() {
    log "${STAGE_LINT}" "Checking for linting tools..."

    local has_linter=false

    if command -v eslint &>/dev/null; then
        log "${STAGE_LINT}" "Running ESLint..."
        if npx eslint . --format=json --output-file="${RESULTS_DIR}/eslint-report.json" 2>/dev/null; then
            log "${STAGE_LINT}" "ESLint: no issues found"
        else
            log_warn "ESLint found issues (see eslint-report.json)"
        fi
        has_linter=true
    fi

    if command -v shellcheck &>/dev/null; then
        log "${STAGE_LINT}" "Running ShellCheck on shell scripts..."
        local shell_files=()
        while IFS= read -r -d '' f; do
            shell_files+=("$f")
        done < <(find "${PIPELINE_ROOT}" -name "*.sh" -print0 2>/dev/null)

        if [ ${#shell_files[@]} -gt 0 ]; then
            if shellcheck "${shell_files[@]}" 2>/dev/null; then
                log "${STAGE_LINT}" "ShellCheck: no issues found"
            else
                log_warn "ShellCheck found issues in shell scripts"
            fi
        fi
        has_linter=true
    fi

    if command -v prettier &>/dev/null || npx prettier --version &>/dev/null; then
        log "${STAGE_LINT}" "Running Prettier format check..."
        npx prettier --check . 2>/dev/null || log_warn "Prettier: formatting issues found"
        has_linter=true
    fi

    if [ "${has_linter}" = false ]; then
        log_warn "No linters found — simulating lint pass"
        log "${STAGE_LINT}" "Simulating: checking for common issues in YAML and shell files..."

        local yaml_files=()
        while IFS= read -r -d '' f; do
            yaml_files+=("$f")
        done < <(find "${PIPELINE_ROOT}" -name "*.yaml" -o -name "*.yml" -print0 2>/dev/null)

        for f in "${yaml_files[@]}"; do
            if python3 -c "import yaml; yaml.safe_load(open('${f}'))" 2>/dev/null; then
                log "${STAGE_LINT}" "YAML valid: ${f}"
            else
                log_error "Invalid YAML: ${f}"
                return 1
            fi
        done
    fi

    log "${STAGE_LINT}" "Checking for common anti-patterns..."
    local anti_pattern_count=0

    if git -C "${PIPELINE_ROOT}" log --oneline -1 2>/dev/null | grep -qiE '(wip|hack|todo|fixme)'; then
        log_warn "Latest commit contains WIP/HACK/TODO/FIXME marker"
        anti_pattern_count=$((anti_pattern_count + 1))
    fi

    if [ ${#yaml_files[@]} -gt 0 ]; then
        for f in "${yaml_files[@]}"; do
            if grep -qE 'password|secret|api.key|token' "${f}" 2>/dev/null; then
                log_error "Potential secret found in: ${f}"
                return 1
            fi
        done
    fi

    if [ ${anti_pattern_count} -eq 0 ]; then
        log "${STAGE_LINT}" "No anti-patterns detected"
    fi

    return 0
}

stage_test() {
    log "${STAGE_TEST}" "Running test stage..."

    local test_framework=false

    if [ -f "${PIPELINE_ROOT}/package.json" ] && grep -q '"test"' "${PIPELINE_ROOT}/package.json"; then
        log "${STAGE_TEST}" "Detected npm test script — running..."
        if npm test 2>/dev/null; then
            log "${STAGE_TEST}" "npm test: all tests passed"
        else
            log_error "npm test: some tests failed"
            return 1
        fi
        test_framework=true
    fi

    if [ "${test_framework}" = false ]; then
        log_warn "No test framework detected — running pipeline validation tests"

        log "${STAGE_TEST}" "Test 1: Verify pipeline configuration exists"
        if [ -f "${PIPELINE_ROOT}/code/ci.yaml" ]; then
            log "${STAGE_TEST}" "  PASS: ci.yaml found"
        else
            log_error "  FAIL: ci.yaml not found"
            return 1
        fi

        log "${STAGE_TEST}" "Test 2: Verify pipeline has required stages"
        local required_stages=("lint" "test" "build" "deploy")
        for stage in "${required_stages[@]}"; do
            if grep -q "${stage}" "${PIPELINE_ROOT}/code/ci.yaml" 2>/dev/null; then
                log "${STAGE_TEST}" "  PASS: stage '${stage}' defined in pipeline"
            else
                log_error "  FAIL: stage '${stage}' missing from pipeline"
                return 1
            fi
        done

        log "${STAGE_TEST}" "Test 3: Verify ci.yaml is valid YAML"
        if python3 -c "import yaml; yaml.safe_load(open('${PIPELINE_ROOT}/code/ci.yaml'))" 2>/dev/null; then
            log "${STAGE_TEST}" "  PASS: ci.yaml is valid YAML"
        else
            log_error "  FAIL: ci.yaml is not valid YAML"
            return 1
        fi

        log "${STAGE_TEST}" "Test 4: Verify caching is configured"
        if grep -qE 'cache:' "${PIPELINE_ROOT}/code/ci.yaml" 2>/dev/null; then
            log "${STAGE_TEST}" "  PASS: Caching configuration found"
        else
            log_warn "  WARN: No caching configuration found (pipelines will be slower)"
        fi

        log "${STAGE_TEST}" "Test 5: Verify deployment strategy"
        if grep -qE 'canary|blue.green|rolling' "${PIPELINE_ROOT}/code/ci.yaml" 2>/dev/null; then
            log "${STAGE_TEST}" "  PASS: Deployment strategy configured"
        else
            log_warn "  WARN: No explicit deployment strategy found"
        fi

        log "${STAGE_TEST}" "Test 6: Verify concurrency control"
        if grep -qE 'concurrency:' "${PIPELINE_ROOT}/code/ci.yaml" 2>/dev/null; then
            log "${STAGE_TEST}" "  PASS: Concurrency control configured"
        else
            log_warn "  WARN: No concurrency group configured (may waste resources)"
        fi
    fi

    return 0
}

stage_build() {
    log "${STAGE_BUILD}" "Running build stage..."

    if [ -f "${PIPELINE_ROOT}/package.json" ] && grep -q '"build"' "${PIPELINE_ROOT}/package.json"; then
        log "${STAGE_BUILD}" "Detected npm build script — running..."
        if npm run build 2>/dev/null; then
            log "${STAGE_BUILD}" "npm build: completed"
        else
            log_error "npm build: failed"
            return 1
        fi
    else
        log "${STAGE_BUILD}" "No build script found — simulating build process"

        log "${STAGE_BUILD}" "Step 1: Cleaning previous builds"
        rm -rf "${RESULTS_DIR}/build" 2>/dev/null || true
        mkdir -p "${RESULTS_DIR}/build"

        log "${STAGE_BUILD}" "Step 2: Assembling artefact metadata"
        cat > "${RESULTS_DIR}/build/metadata.json" <<EOF
{
  "version": "$(git -C "${PIPELINE_ROOT}" describe --tags --always 2>/dev/null || echo '0.0.0-dev')",
  "commit": "$(git -C "${PIPELINE_ROOT}" rev-parse --short HEAD 2>/dev/null || echo 'unknown')",
  "branch": "$(git -C "${PIPELINE_ROOT}" branch --show-current 2>/dev/null || echo 'unknown')",
  "build_timestamp": "$(date -u '+%Y-%m-%dT%H:%M:%SZ')",
  "build_runner": "local-ci-simulator"
}
EOF

        log "${STAGE_BUILD}" "Step 3: Validating artefact"
        if python3 -c "import json; json.load(open('${RESULTS_DIR}/build/metadata.json'))" 2>/dev/null; then
            log "${STAGE_BUILD}" "Artefact metadata is valid JSON"
        else
            log_error "Artefact metadata is invalid"
            return 1
        fi

        log "${STAGE_BUILD}" "Step 4: Computing checksums"
        if command -v shasum &>/dev/null; then
            shasum -a 256 "${RESULTS_DIR}/build/metadata.json" > "${RESULTS_DIR}/build/metadata.json.sha256"
            log "${STAGE_BUILD}" "SHA256: $(cut -d' ' -f1 < "${RESULTS_DIR}/build/metadata.json.sha256")"
        else
            log_warn "shasum not available — skipping checksum"
        fi

        log "${STAGE_BUILD}" "Step 5: Archiving build artefact"
        tar -czf "${RESULTS_DIR}/build/artefact.tar.gz" \
            -C "${RESULTS_DIR}/build" \
            metadata.json 2>/dev/null || \
            log_warn "Could not create tar archive (non-critical)"

        log "${STAGE_BUILD}" "Build completed — artefacts in ${RESULTS_DIR}/build/"
    fi

    return 0
}

stage_deploy() {
    log "${STAGE_DEPLOY}" "Running deploy stage (simulation)..."

    local environment="${DEPLOY_ENV:-staging}"
    log "${STAGE_DEPLOY}" "Target environment: ${environment}"

    log "${STAGE_DEPLOY}" "Step 1: Pre-deployment health check"
    local health_check_url=""
    case "${environment}" in
        production)  health_check_url="https://example.com/health" ;;
        staging)     health_check_url="https://staging.example.com/health" ;;
        *)           health_check_url="https://localhost:8080/health" ;;
    esac

    log "${STAGE_DEPLOY}" "  Checking ${health_check_url}..."
    if curl -sf --max-time 5 "${health_check_url}" &>/dev/null; then
        log "${STAGE_DEPLOY}" "  Health check passed"
    else
        log_warn "  Health check unreachable (expected for simulation)"
    fi

    log "${STAGE_DEPLOY}" "Step 2: Deploying artefacts"
    log "${STAGE_DEPLOY}" "  Uploading build to ${environment}..."
    echo "  [simulated] s3 sync ${RESULTS_DIR}/build/ s3://${environment}-bucket/"

    log "${STAGE_DEPLOY}" "Step 3: Running smoke tests"
    echo "  [simulated] curl -sf ${health_check_url}"
    log "${STAGE_DEPLOY}" "  Smoke test: PASSED (simulated)"

    log "${STAGE_DEPLOY}" "Step 4: Verifying deployment"
    if [ -f "${RESULTS_DIR}/build/metadata.json" ]; then
        local deploy_version
        deploy_version=$(python3 -c "import json; print(json.load(open('${RESULTS_DIR}/build/metadata.json'))['version'])" 2>/dev/null || echo "unknown")
        log "${STAGE_DEPLOY}" "  Deployed version: ${deploy_version}"
    fi

    log "${STAGE_DEPLOY}" "Step 5: Post-deployment notification"
    echo "  [simulated] Slack notification: '${environment} deployment complete'"

    log "${STAGE_DEPLOY}" "Deploy simulation completed for ${environment}"
    return 0
}

stage_validate() {
    log "${STAGE_VALIDATE}" "Running pipeline validation..."

    local pipeline_file="${PIPELINE_ROOT}/code/ci.yaml"

    log "${STAGE_VALIDATE}" "Check 1: Pipeline configuration exists"
    if [ ! -f "${pipeline_file}" ]; then
        log_error "Pipeline file not found: ${pipeline_file}"
        return 1
    fi
    log "${STAGE_VALIDATE}" "  Found: ${pipeline_file}"

    log "${STAGE_VALIDATE}" "Check 2: Pipeline has a name"
    if grep -qE '^name:' "${pipeline_file}"; then
        local name
        name=$(grep '^name:' "${pipeline_file}" | head -1 | sed 's/^name: *//')
        log "${STAGE_VALIDATE}" "  Pipeline name: ${name}"
    else
        log_warn "  Pipeline has no name (best practice: always name your pipeline)"
    fi

    log "${STAGE_VALIDATE}" "Check 3: Pipeline triggers are defined"
    if grep -qE '^on:' "${pipeline_file}"; then
        log "${STAGE_VALIDATE}" "  Triggers are defined"
    else
        log_error "  No triggers defined"
        return 1
    fi

    log "${STAGE_VALIDATE}" "Check 4: Fail-fast ordering (lint before test before build)"
    local lint_line test_line build_line
    lint_line=$(grep -n 'lint' "${pipeline_file}" | head -1 | cut -d: -f1 || echo "999")
    test_line=$(grep -n 'test' "${pipeline_file}" | head -1 | cut -d: -f1 || echo "999")
    build_line=$(grep -n 'build' "${pipeline_file}" | head -1 | cut -d: -f1 || echo "999")

    if [ "${lint_line}" -lt "${test_line}" ] && [ "${test_line}" -lt "${build_line}" ]; then
        log "${STAGE_VALIDATE}" "  Correct ordering: lint → test → build"
    else
        log_warn "  Suboptimal ordering: consider putting faster checks first"
    fi

    log "${STAGE_VALIDATE}" "Check 5: No hardcoded secrets in pipeline"
    if grep -qiE '(password|secret|api_key|token).*:.*["'"'"'][A-Za-z0-9]' "${pipeline_file}" 2>/dev/null; then
        log_error "  Potential hardcoded secret found in pipeline configuration!"
        return 1
    else
        log "${STAGE_VALIDATE}" "  No hardcoded secrets detected"
    fi

    log "${STAGE_VALIDATE}" "Check 6: Concurrency control present"
    if grep -qE 'concurrency:' "${pipeline_file}"; then
        log "${STAGE_VALIDATE}" "  Concurrency control: configured"
    else
        log_warn "  No concurrency control (may waste resources on rapid pushes)"
    fi

    log "${STAGE_VALIDATE}" "Check 7: Caching configured"
    if grep -qE 'cache:' "${pipeline_file}"; then
        log "${STAGE_VALIDATE}" "  Caching: configured"
    else
        log_warn "  No dependency caching (pipeline will be slower)"
    fi

    log "${STAGE_VALIDATE}" "Check 8: Deployment strategy"
    if grep -qiE '(canary|blue.green|rolling|staging|production)' "${pipeline_file}"; then
        log "${STAGE_VALIDATE}" "  Deployment strategy: detected"
    else
        log_warn "  No deployment strategy configured"
    fi

    log "${STAGE_VALIDATE}" "Check 9: Environment protection"
    if grep -qE 'environment:' "${pipeline_file}"; then
        log "${STAGE_VALIDATE}" "  Environment protection: configured"
    else
        log_warn "  No environment protection rules"
    fi

    log "${STAGE_VALIDATE}" "Check 10: Rollback mechanism"
    if grep -qiE 'rollback' "${pipeline_file}"; then
        log "${STAGE_VALIDATE}" "  Rollback: defined"
    else
        log_warn "  No rollback mechanism defined"
    fi

    log "${STAGE_VALIDATE}" "Pipeline validation completed"
    return 0
}

print_summary() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  PIPELINE SUMMARY${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "  Total stages:  ${TOTAL_STAGES}"
    echo -e "  ${GREEN}Passed:  ${PASSED_STAGES}${NC}"
    echo -e "  ${RED}Failed:  ${FAILED_STAGES}${NC}"
    echo -e "  ${YELLOW}Skipped: ${SKIPPED_STAGES}${NC}"
    echo ""
    echo -e "  Full log: ${PIPELINE_LOG}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    if [ ${FAILED_STAGES} -gt 0 ]; then
        echo -e "\n${RED}Pipeline FAILED. Fix the issues above and re-run.${NC}\n"
        return 1
    else
        echo -e "\n${GREEN}Pipeline PASSED. All stages green.${NC}\n"
        return 0
    fi
}

main() {
    local start_time
    start_time=$(date +%s)

    echo -e "${CYAN}"
    echo "  ╔══════════════════════════════════════════════╗"
    echo "  ║     Local CI/CD Pipeline Simulator v1.0      ║"
    echo "  ║     Build & CI/CD — Pipelines That Don't Suck║"
    echo "  ╚══════════════════════════════════════════════╝"
    echo -e "${NC}"

    mkdir -p "${RESULTS_DIR}"
    echo "Pipeline run: $(date -u '+%Y-%m-%dT%H:%M:%SZ')" > "${PIPELINE_LOG}"

    local stages=("${STAGE_LINT}" "${STAGE_TEST}" "${STAGE_BUILD}" "${STAGE_DEPLOY}" "${STAGE_VALIDATE}")

    run_stage "${STAGE_LINT}" stage_lint

    if [ ${FAILED_STAGES} -eq 0 ]; then
        run_stage "${STAGE_TEST}" stage_test
    else
        log_warn "Skipping test stage due to lint failure (fail-fast)"
        SKIPPED_STAGES=$((SKIPPED_STAGES + 3))
    fi

    if [ ${FAILED_STAGES} -eq 0 ]; then
        run_stage "${STAGE_BUILD}" stage_build
    elif [ ${SKIPPED_STAGES} -eq 0 ]; then
        log_warn "Skipping build stage due to test failure (fail-fast)"
        SKIPPED_STAGES=$((SKIPPED_STAGES + 2))
    fi

    if [ ${FAILED_STAGES} -eq 0 ]; then
        run_stage "${STAGE_DEPLOY}" stage_deploy
    elif [ ${SKIPPED_STAGES} -eq 0 ]; then
        log_warn "Skipping deploy stage due to build failure (fail-fast)"
        SKIPPED_STAGES=$((SKIPPED_STAGES + 1))
    fi

    run_stage "${STAGE_VALIDATE}" stage_validate

    local end_time
    end_time=$(date +%s)
    local total_duration=$((end_time - start_time))
    echo ""
    log_info "Total pipeline duration: ${total_duration}s"

    print_summary
    return ${EXIT_CODE}
}

main "$@"