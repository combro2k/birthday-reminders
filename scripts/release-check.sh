#!/usr/bin/env bash
set -euo pipefail

printf '\n[release-check] Running cargo fmt --all -- --check\n'
cargo fmt --all -- --check

printf '\n[release-check] Running cargo test --all-targets --all-features\n'
cargo test --all-targets --all-features

printf '\n[release-check] Running cargo clippy --all-targets --all-features -- -D warnings\n'
cargo clippy --all-targets --all-features -- -D warnings

printf '\n[release-check] All checks passed.\n'
