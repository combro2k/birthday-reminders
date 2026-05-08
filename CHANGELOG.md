# Changelog

All notable changes to this project must be documented in this file.

The format is based on Keep a Changelog and this project uses semantic versioning.

## [Unreleased]

### Added

### Changed
- Updated default runtime and install paths to use system locations consistently:
  - Default CLI config path is now `/etc/birthday-reminders/config.yaml`
  - `package/install.sh`, `package/uninstall.sh`, and `Makefile` now default to `/usr/bin`, `/etc/birthday-reminders`, and `/var/lib/birthday-reminders`
  - Default SQLite URL in `config.yaml.example` now points to `/var/lib/birthday-reminders/birthday_reminders.db?mode=rwc`
- Updated README examples and operational docs to match the system-path defaults above

### Fixed

### Security

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
