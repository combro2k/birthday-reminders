#!/usr/bin/env bash
set -euo pipefail

printf '\n[release-check] Verifying version consistency between Cargo.toml and package.json\n'
cargo_version="$(grep -E '^version\s*=\s*"' Cargo.toml | head -n1 | sed -E 's/^version\s*=\s*"([^"]+)".*/\1/')"
package_version="$(grep -E '"version"\s*:' package.json | head -n1 | sed -E 's/.*"version"\s*:\s*"([^"]+)".*/\1/')"

if [[ -z "${cargo_version}" || -z "${package_version}" ]]; then
	printf '[release-check] ERROR: Could not determine version from Cargo.toml or package.json\n' >&2
	exit 1
fi

if [[ "${cargo_version}" != "${package_version}" ]]; then
	printf '[release-check] ERROR: Version mismatch detected (Cargo.toml=%s, package.json=%s)\n' "${cargo_version}" "${package_version}" >&2
	exit 1
fi

printf '[release-check] Version check passed (%s)\n' "${cargo_version}"

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
	changed_for_clean="$(git status --porcelain -- src static templates tests migrations)"
	if [[ -n "${changed_for_clean}" ]]; then
		printf '\n[release-check] Relevant changes detected (src/static/templates/tests/migrations); running cargo clean\n'
		cargo clean
	else
		printf '\n[release-check] No relevant changes in src/static/templates/tests/migrations; skipping cargo clean\n'
	fi
else
	printf '\n[release-check] Git repository context unavailable; running cargo clean\n'
	cargo clean
fi

printf '\n[release-check] Running gitleaks detect\n'
gitleaks detect

printf '\n[release-check] Running Tailwind CSS build\n'
npx @tailwindcss/cli -i ./static/tailwind.input.css -o ./static/tailwind.css --minify

printf '\n[release-check] Running cargo fmt --all -- --check\n'
cargo fmt --all -- --check

printf '\n[release-check] Running cargo test --all-targets --all-features\n'
cargo test --all-targets --all-features

printf '\n[release-check] Running cargo clippy --all-targets --all-features -- -D warnings\n'
cargo clippy --all-targets --all-features -- -D warnings

printf '\n[release-check] All checks passed.\n'
