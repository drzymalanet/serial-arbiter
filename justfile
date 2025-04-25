# Run clippy linter
lint:
    cargo clippy --workspace -- --deny warnings --deny unused_crate_dependencies

