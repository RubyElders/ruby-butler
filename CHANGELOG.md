# Changelog

All notable changes to Ruby Butler will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-03-03

### Added
- Bash shell completion with context-aware suggestions via `rb shell-integration bash`
- Working directory flag `-C, --work-dir` to run commands from a different directory
- `rb info` diagnostic subcommands: `runtime`, `env`, `project`, `config`
- `rb version` command with git hash, build profile, and dirty state display
- Custom help system with grouped command display and gentleman's formatting
- Sophisticated error display with platform-specific guidance for bundler failures
- Regression test for bundler deprecation warnings
- macOS ARM64 (aarch64) release builds
- Shell script linting (ShellCheck) and YAML linting in CI
- PowerShell script linting (PSScriptAnalyzer) in CI

### Changed
- Rewritten CLI structure with command dispatch, separated help, and refined output
- Ruby detection rewritten with strategy pattern (version file, Gemfile, CLI flag)
- Gem path detection split into focused strategies (bundler isolation, custom base, user gems)
- Bundler sync now cleans removed gems from lockfile automatically
- Updated `bundle config path` to `bundle config set path` for modern Bundler compatibility
- Test environment upgraded to Ruby 4.0.1 + 3.4.5 (from 3.2.4 + 3.4.5)
- CI now builds Docker image locally on PRs when Dockerfile changes

### Fixed
- Bundler deprecation warning with Ruby 4.0+ (`bundle config` → `bundle config set`)

## [0.2.0] - 2025-10-19

### Added
- Configuration file support via `rb.toml` or `rb.kdl` for global settings
- Project script management with `rbproject.toml` or `rbproject.kdl` and `rb run` command
- Project bootstrap script with `rb init`
- Alternative project file naming: `gem.toml` or `gem.kdl` (for gem development)
- Short flag `-B` for `--no-bundler` option

## [0.1.0] - 2025-09-26

### Added
- Core Ruby environment detection and orchestration
- Ruby runtime discovery with semver-based version selection
- Gem environment management with proper PATH composition
- Bundler project detection and automatic `bundle exec` integration
- Distinguished command-line interface with gentleman's butler persona
- Cross-platform support (Windows, macOS, Linux)
- Environment composition without shell modification
- Comprehensive error handling with sophisticated messaging
- Git version embedding in binaries for traceability
- Automated release workflow with cross-platform binaries
- Complete test suite with unit, integration, and shell tests

---

*Distinguished releases crafted with appropriate ceremony by RubyElders.com*
