# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.3] - 2025-12-21

### Added
- **Releases**:
  - Added github workflow file to cross compile binaries for windows, linux and macos and add artifacts to release on tag push

## [1.0.2] - 2025-12-21

### Added
- **Tests**:
  - Added system tests to the project.
  - Added integration tests to the project.
  - Added unit tests for core logic (piece, tree, models, scanner, edit, verify, hashing).

## [1.0.1] - 2025-12-21

### Added
- **New Subcommands**:
  - `verify`: Verify local files against a torrent file.
  - `edit`: Modify an existing torrent's metadata (announce URLs, comment, private flag).
- **CLI Improvements**:
  - `--json`: Output results in JSON format.
  - `--info-hash`: Display the info hash of the created torrent.
  - `--date`: Set the creation date (Unix timestamp) manually.
  - Support for `SOURCE_DATE_EPOCH` for reproducible builds.
  - Progress bar support during torrent creation.
  - Colored terminal output for better readability.
- **Dependencies**: Added `console`, `indicatif`, `hex`, and `urlencoding`.

### Changed
- **CLI Structure**: Refactored to use subcommands (`create`, `verify`, `edit`). The default behavior is `create` if no subcommand is provided.
- **Binary Name**: Updated application name to `torrite` in help messages.
- **Output**: Improved logging and verbose output format.
