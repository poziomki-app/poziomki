# Quality Rules
- Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Rust metrics: run `backend/scripts/rust-code-analysis.sh` and keep it passing.
- Kotlin: `./gradlew ktlintCheck detekt` in `mobile/`.
- Treat warnings as errors; do not merge if any quality gate fails.
- Never bypass checks in CI; fix code or adjust thresholds in PR with justification.
