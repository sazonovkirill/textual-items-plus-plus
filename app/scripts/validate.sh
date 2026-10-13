#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Tests always run in parallel (docs/testing-strategy.md, Parallel execution).
if [[ -n "${RUST_TEST_THREADS:-}" ]]; then
  echo 'validate: unset RUST_TEST_THREADS; tests always run in parallel' >&2
  exit 1
fi

format() {
  cargo fmt --all
}

static_checks() {
  cargo fmt --all --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo deny --locked check
  bash -n scripts/validate.sh
  bash -n scripts/review.sh
  layers
  pending_snapshots
}

# Domain and App are the UI-independent core (docs/code-quality.md, Architecture).
layers() {
  local found=0
  if [[ -d src/domain ]] && grep -rnwE 'ratatui|crossterm|app|ui' src/domain | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//'; then
    echo 'layers: src/domain may depend only on std and small pure crates, never on ratatui, crossterm, app or ui' >&2
    found=1
  fi
  if [[ -d src/app ]] && grep -rnwE 'ratatui|crossterm|ui' src/app | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//'; then
    echo 'layers: src/app may depend only on domain; turn key events into app messages and render in src/ui' >&2
    found=1
  fi
  return "$found"
}

# Pending insta snapshots are never committed (docs/testing-strategy.md, Snapshots).
pending_snapshots() {
  local pending
  pending="$(find . -path ./target -prune -o \( -name '*.snap.new' -o -name '*.pending-snap' \) -print)"
  if [[ -n "$pending" ]]; then
    echo "$pending" >&2
    echo 'snapshots: review these with cargo insta review before validating' >&2
    return 1
  fi
}

unit() {
  RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --locked --all-targets --all-features
}

# Every line outside terminal setup in main.rs must be covered (docs/testing-strategy.md).
coverage() {
  if [[ ! -f src/lib.rs ]]; then
    echo 'coverage: skipped, no src/lib.rs yet'
    return
  fi
  cargo llvm-cov --locked --all-targets --all-features \
    --ignore-filename-regex 'src/main\.rs$' --fail-under-lines 100
}

# Times the performance requirements (N-PERF-…) in release mode (docs/testing-strategy.md, Performance).
bench() {
  cargo bench --locked --bench perf
}

# Checks the prerequisites in docs/agent-workflow.md and prints how to fix each missing one.
doctor() {
  local missing=0 rustup_bin toolchain installed component tool
  need() {
    if ! command -v "$1" >/dev/null 2>&1; then
      echo "doctor: missing $1; $2" >&2
      missing=1
    fi
  }
  need git 'install Xcode Command Line Tools: xcode-select --install'
  need openssl 'brew install openssl'
  need claude 'install Claude Code: https://code.claude.com/docs/en/setup'
  # Cloud sessions run without Codex, which reviews milestones on the owner's Mac, and get Todoist as a connector.
  local cloud=0
  [[ "${CLAUDE_CODE_REMOTE:-}" == true ]] && cloud=1
  ((cloud)) || need codex 'install Codex CLI: https://developers.openai.com/codex/cli'
  need rustup 'install rustup: https://rustup.rs'
  need jq 'brew install jq'
  if ((!cloud)) && command -v claude >/dev/null 2>&1 && ! claude mcp get todoist >/dev/null 2>&1; then
    echo 'doctor: Claude Code has no todoist MCP server; claude mcp add --transport http --scope user todoist https://ai.todoist.net/mcp' >&2
    missing=1
  fi
  if ((!cloud)) && command -v codex >/dev/null 2>&1 && ! codex mcp get todoist >/dev/null 2>&1; then
    echo 'doctor: Codex has no todoist MCP server; codex mcp add todoist --url https://ai.todoist.net/mcp' >&2
    missing=1
  fi
  if rustup_bin="$(command -v rustup)"; then
    if [[ "$(command -v cargo)" != "$(dirname "$rustup_bin")/cargo" ]]; then
      echo "doctor: cargo resolves to $(command -v cargo), not the rustup proxy; put $(dirname "$rustup_bin") first on PATH" >&2
      missing=1
    fi
    if ! toolchain="$(rustup show active-toolchain 2>/dev/null)"; then
      echo 'doctor: the pinned Rust toolchain is not installed; rustup toolchain install' >&2
      missing=1
    elif [[ "$toolchain" != "$(sed -n 's/^channel = "\(.*\)"$/\1/p' ../rust-toolchain.toml)"-* ]]; then
      echo "doctor: active toolchain is $toolchain, not the release pinned in rust-toolchain.toml; unset RUSTUP_TOOLCHAIN and run rustup override unset" >&2
      missing=1
    elif ! installed="$(rustup component list --installed 2>/dev/null)"; then
      echo 'doctor: the pinned Rust toolchain is not installed; rustup toolchain install' >&2
      missing=1
    else
      for component in rustfmt clippy llvm-tools rust-src; do
        if ! grep -q "^$component" <<<"$installed"; then
          echo "doctor: missing Rust component $component; rustup component add $component" >&2
          missing=1
        fi
      done
    fi
  fi
  for tool in llvm-cov deny; do
    if ! cargo "$tool" --version >/dev/null 2>&1; then
      echo "doctor: missing cargo-$tool; cargo install --locked cargo-$tool" >&2
      missing=1
    fi
  done
  if ((missing)); then
    return 1
  fi
  echo 'doctor: all prerequisites found'
}

# Runs the real-user scenarios against the real binary, those whose path contains a filter when any is given.
scenarios() {
  cargo test --locked --test scenarios -- "$@"
}

# Runs the key-sequence fuzzer far longer than `all` does, on a fresh seed unless PROPTEST_RNG_SEED names one.
fuzz() {
  local seed="${PROPTEST_RNG_SEED:-$(od -An -N4 -tu4 /dev/urandom | tr -d ' ')}"
  echo "fuzz: seed $seed; rerun with PROPTEST_RNG_SEED=$seed"
  PROPTEST_RNG_SEED="$seed" PROPTEST_CASES="${PROPTEST_CASES:-1000}" cargo test --locked --release --test ui fuzz::
}

# Lists Must requirements that no test or source file names yet (docs/testing-strategy.md, Traceability).
trace() {
  local id untraced=0 total=0
  while IFS= read -r id; do
    total=$((total + 1))
    if ! grep -rqw -- "$id" src tests 2>/dev/null; then
      echo "$id"
      untraced=$((untraced + 1))
    fi
  done < <(grep -hoE '^\| (FR|F|N)-[A-Z]+-[0-9]+ \|.*\| Must \||^\*\*FR-[A-Z]+-[0-9]+ \(Must\)' ../specs/functional-requirements.md ../specs/requirements.md |
    grep -oE '(FR|F|N)-[A-Z]+-[0-9]+' | awk '!seen[$0]++')
  echo "trace: $untraced of $total Must requirements are not referenced by any test or source file"
}

case "${1:-all}" in
  format) format ;;
  static) static_checks ;;
  test) unit ;;
  coverage) coverage ;;
  doctor) doctor ;;
  trace) trace ;;
  bench) bench ;;
  scenarios) scenarios "${@:2}" ;;
  fuzz) fuzz ;;
  # The coverage run is the test run: llvm-cov runs every test, and clippy above already denies warnings.
  all) static_checks; coverage ;;
  *) echo 'Usage: bash app/scripts/validate.sh [all|static|test|coverage|format|doctor|trace|bench|scenarios [filter...]|fuzz]' >&2; exit 2 ;;
esac
