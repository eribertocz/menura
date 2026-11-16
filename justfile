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

# Run the conformance suite, optionally for one area.
conformance area="":
    @echo "no cases yet"
