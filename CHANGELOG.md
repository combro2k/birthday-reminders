# Changelog

All notable changes to this project must be documented in this file.

The format is based on Keep a Changelog and this project uses semantic versioning.

## [Unreleased]

### Added
- **WhatsApp Cloud API channel**: Users can now send birthday reminders via Meta's WhatsApp Cloud API.
  - Requires a Meta WhatsApp Business Account with a phone number ID and permanent access token.
  - Supports E.164 formatted recipient phone numbers.
  - Includes automatic retry with exponential backoff for transient failures (rate limits and server errors).
  - Per-user/channel rate limiting (500ms minimum interval) prevents burst traffic to WhatsApp API.
  - Test notification endpoint available to verify channel configuration.

### Changed
- Notification channels page now groups providers by category (Email, SMS, Push Notifications, Messaging Apps) to make configuration options easier to scan.
- WhatsApp channel is now fully implemented and appears in the Messaging Apps group (no longer Coming Soon).
- README Notification Channels section updated to include WhatsApp Cloud API setup guide with credential requirements and phone number format documentation.
- README removed manual JSON configuration examples from Proton Mail and Ntfy setup sections; configuration is now done exclusively through the web UI.
- Removed unused `NotificationError::NotImplemented` variant to satisfy strict linting.

### Fixed
- Docker build now includes the previously missing templates build folder during image build, resolving build failures caused by absent template files.

### Security

## [1.1.6] - 2026-05-08

### Changed
- Hardened Docker image build and runtime: switched to a `scratch` runtime stage with non-root execution (`UID/GID 10001`) and locked Rust dependency builds for reproducibility.
- Added explicit CA certificate handling in the container image to preserve outbound TLS functionality for integrations that require HTTPS/TLS.

## [1.1.5] - 2026-05-08

### Changed
- Raised the Rust toolchain baseline to 1.95 across project metadata, documentation, Docker build image, and GitHub Actions workflows

## [1.1.4] - 2026-05-08

### Fixed
- GitHub Actions CI and release workflows now install and invoke sccache in a way that matches the repository's Cargo wrapper configuration, avoiding pipeline failures caused by a missing hard-coded sccache path

## [1.1.3] - 2026-05-08

### Changed
- GitHub Actions CI/Release workflows now install sccache, mold, and clang to match local build configuration

## [1.1.2] - 2026-05-08

### Added
- Ntfy notification channel support for both official ntfy.sh and self-hosted servers
- Pushover notification channel support via the Pushover API (`api_token` + `user_key`)
- Channel list page remove-confirmation behavior moved to `static/channels/list.js` and wired via `page-channels-list`

### Changed
- Ntfy priority header now supports reminder-aware mapping with default value `3` and optional overrides for same-day and next-day reminders
- Notification channel UI and README documentation expanded with Ntfy setup examples, authentication modes, and priority mapping guidance
- Notifications channels overview now renders a single card-based list with per-channel enabled/disabled badges and grouped actions
- Channels handler/template mapping simplified by deriving configured/enabled state directly from channel records per implemented channel kind

### Fixed
- Removed inline `onsubmit` handler from notification channel removal form to keep behavior in external static assets

### Security

## [1.1.1] - 2026-05-08

### Added
- Per-user dashboard upcoming window preference with allowed values 30, 45, 60, 75, and 90 days
- Per-user default sorting preference for All Birthdays (sort field + direction)
- Database migrations for SQLite, MySQL, and PostgreSQL to persist dashboard and birthday sorting preferences
- Unit tests for settings preference validation/parsing and birthdays list sort-resolution behavior

### Changed
- Added explicit Rust package license metadata in Cargo.toml (`license = "MIT"`) to make crate licensing clear in Cargo ecosystem tooling
- Updated `AGENTS.md` version bump workflow policy to commit all changed files (including version-bump files) by default.
- Dashboard upcoming query now uses the authenticated user's configured window instead of a hardcoded 30-day range
- Dashboard empty-state copy now reflects the user's configured upcoming window
- All Birthdays default ordering now uses each user's saved sort preference (default: closest birthdays / shortest days until next)
- All Birthdays query parameters still override saved sort defaults when explicitly provided
- Settings profile now includes forms to configure dashboard upcoming window and default All Birthdays sorting
- Reminder preferences now support additional long-range lead times: 30, 45, 60, 75, and 90 days
- Reminder checkbox handling moved from inline template script to external static module

### Fixed
- `package/install.sh` now explicitly sets `750` permissions on the data and static directories, preventing the SQLite database from being world-readable when installed via the tar.gz package

### Security

## [1.1.0] - 2026-05-08

### Changed
- Updated default runtime and install paths to use system locations consistently:
  - Default CLI config path is now `/etc/birthday-reminders/config.yaml`
  - `package/install.sh`, `package/uninstall.sh`, and `Makefile` now default to `/usr/bin`, `/etc/birthday-reminders`, and `/var/lib/birthday-reminders`
  - Default SQLite URL in `config.yaml.example` now points to `/var/lib/birthday-reminders/birthday_reminders.db?mode=rwc`
- Updated README examples and operational docs to match the system-path defaults above
- Updated `scripts/release-check.sh` to run `cargo clean` conditionally, only when Git changes are present in `src/`, `static/`, `templates/`, `tests/`, or `migrations/`
- Updated `AGENTS.md` and `README.md` to document the conditional `cargo clean` behavior in the release-check workflow
- Updated `AGENTS.md` release guidance so version bumps without an explicit target suggest next minor by default (with next major as alternative) and always require confirmation before applying

## [1.0.6] - 2026-05-08

### Changed
- Added `cargo clean` to release-check script for more consistent builds

## [1.0.5] - 2026-05-08

### Changed
- Updated README.md to include mold, sccache, and clang in Build & Development Tools prerequisites

## [1.0.4] - 2026-05-08

### Added
- Secret scanning with gitleaks integrated into the release check script
- Gitleaks validation step in GitHub Actions CI pipeline

### Changed
- Enhanced Prerequisites section in README with categorized build and development tools
- Expanded documentation of release checklist with detailed step descriptions

## [1.0.3] - 2026-05-08

### Added
- GitHub Actions release workflow for automated .deb and .apk package builds on version tags
- Makefile targets for local package generation: `make package-deb`, `make package-apk`, `make packages`
- nfpm configuration for Debian and Alpine Linux packaging with FHS-compliant paths
- Package lifecycle scripts (postinstall, preremove) for both Debian and Alpine

### Changed
- Switched binary and configuration paths to follow Filesystem Hierarchy Standard (FHS)
  - Binary moved from `/opt/birthday-reminders/bin/` to `/usr/bin/`
  - Configuration moved from `/opt/birthday-reminders/etc/` to `/etc/birthday-reminders/`
  - Data directory moved from `/opt/birthday-reminders/data/` to `/var/lib/birthday-reminders/`
- Updated systemd and OpenRC service files to use new FHS paths
- Existing `make package` command renamed to `make package-tar` for tarball generation

### Fixed
- Dockerfile missing `COPY static/ static/` in builder stage, required for rust-embed compilation

## [1.0.2] - 2026-05-08

### Changed
- Switched to a dark-first theme baseline with explicit light mode styling.
- Set dark mode as the default theme for new users and default template rendering.
- Updated theme settings copy to make dark default behavior explicit while keeping auto mode support.

### Fixed
- Reduced Android refresh flicker by applying theme classes before stylesheet paint.

## [1.0.1] - 2026-05-08

### Changed
- Project maintenance release with workflow and release-process updates.

## [0.0.0] - 2026-05-08

### Added
- Initial changelog file.
