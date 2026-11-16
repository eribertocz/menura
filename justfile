# Development recipes. CI runs the same commands (spec/09-continuous-integration.md CI-1).

# Check formatting.
fmt:
    cargo fmt --check

# Run clippy with every warning denied.
lint:
    cargo clippy --all-targets -- -D warnings

# Run every test.
test:
    cargo test

# Check dependencies: allowlist, banned crates, licenses, sources and advisories.
deny:
    cargo deny check

# Run the conformance suite, optionally for one area (CS-4.1).
conformance area="":
    cargo run --locked --quiet -p semantic-compiler-conformance-runner -- conformance "{{area}}"

# List rule IDs without a conformance case, CI check or review item (T1); fails only for the
# files of spec/coverage-scope.txt (CS-2.6). Requires PyYAML.
rule-coverage:
    python3 scripts/ci/check_rule_coverage.py
